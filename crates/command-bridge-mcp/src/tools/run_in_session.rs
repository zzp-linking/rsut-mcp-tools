use serde_json::Value;
use std::io::Read;
use std::os::windows::process::CommandExt;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;
use crate::session::store;
use crate::tools::run_command::{check_forbidden, INTERACTIVE_COMMANDS};

/// 注入到命令末尾的 cwd 捕获标记，足够独特以避免误匹配
const CWD_MARKER: &str = "__MCP_CWD_CAPTURE_7f3a__";

pub fn run_in_session(args: Value) -> String {
    let session_id = match args.get("session_id").and_then(|v| v.as_str()) {
        Some(id) => id.to_string(),
        None => return "错误：缺少必填参数 session_id".to_string(),
    };

    let command = match args.get("command").and_then(|v| v.as_str()) {
        Some(c) if !c.trim().is_empty() => c.trim().to_string(),
        _ => return "错误：缺少必填参数 command".to_string(),
    };

    // 禁止删除命令
    if let Some(err) = check_forbidden(&command) {
        return err;
    }

    // 交互式命令检测
    let first_token = command.split_whitespace().next().unwrap_or("").to_lowercase();
    let exe_name = first_token.trim_end_matches(".exe").trim_end_matches(".cmd");
    if INTERACTIVE_COMMANDS.contains(&exe_name) {
        return format!(
            "错误：命令 \"{}\" 是交互式程序，不支持执行。请改用非交互模式（如 git log、git diff 等）。",
            exe_name
        );
    }

    let timeout_secs = args
        .get("timeout_secs")
        .and_then(|v| v.as_u64())
        .unwrap_or(30);

    // 读取 session 状态（cwd + shell）
    let (session_cwd, shell) = {
        let guard = store().lock().unwrap();
        match guard.get(&session_id) {
            Some(s) => (s.cwd.clone(), s.shell.clone()),
            None => return format!(
                "错误：session \"{}\" 不存在，请先调用 open_session 创建。",
                session_id
            ),
        }
    };

    eprintln!(
        "[DEBUG] run_in_session: id={}, shell={}, cwd={}, cmd={}",
        session_id, shell, session_cwd, command
    );

    let child = spawn_command(&shell, &session_cwd, &command);
    let child = match child {
        Ok(c) => c,
        Err(e) => return format!("错误：启动进程失败: {}", e),
    };

    let (exit_code, raw_stdout, raw_stderr) = match wait_with_timeout(child, Duration::from_secs(timeout_secs)) {
        WaitResult::Finished { exit_code, stdout_bytes, stderr_bytes } => {
            (exit_code, stdout_bytes, stderr_bytes)
        }
        WaitResult::Timeout => {
            return format!("错误：命令执行超时（{}秒），进程已终止", timeout_secs);
        }
        WaitResult::Error(e) => {
            return format!("错误：等待进程失败: {}", e);
        }
    };

    let stdout_str = decode_bytes(&raw_stdout);
    let stderr_str = decode_bytes(&raw_stderr);

    // 从 stdout 末尾解析新 cwd
    let (user_output, new_cwd) = extract_cwd_from_output(&stdout_str, &session_cwd);

    // 更新 session 的 cwd
    {
        let mut guard = store().lock().unwrap();
        if let Some(s) = guard.get_mut(&session_id) {
            s.cwd = new_cwd.clone();
        }
    }

    // 组装返回文本
    let mut output = String::new();
    output.push_str(&format!("session_id: {}", session_id));
    output.push_str(&format!("\ncwd: {}", new_cwd));
    output.push_str(&format!("\nexit_code: {}", exit_code));

    let stdout_clean = user_output.trim_end();
    if !stdout_clean.is_empty() {
        output.push_str("\n\n--- stdout ---\n");
        output.push_str(stdout_clean);
    }

    let stderr_clean = stderr_str.trim_end();
    if !stderr_clean.is_empty() {
        output.push_str("\n\n--- stderr ---\n");
        output.push_str(stderr_clean);
    }

    if stdout_clean.is_empty() && stderr_clean.is_empty() {
        output.push_str("\n\n(无输出)");
    }

    output
}

/// 启动子进程：
/// - 用 current_dir() 设置工作目录（走 OS API，不经过 cmd 路径解析，无引号问题）
/// - 用 raw_arg() 传命令字符串（绕过 Rust 的自动引号转义，让 cmd/powershell 原样收到）
/// - 末尾注入 cwd 捕获，不在命令字符串里写 cd /D
fn spawn_command(
    shell: &str,
    cwd: &str,
    command: &str,
) -> std::io::Result<std::process::Child> {
    match shell {
        "powershell" | "ps" => {
            // PowerShell：用 $PWD 输出当前路径
            let script = format!(
                "$OutputEncoding = [System.Text.Encoding]::UTF8; {}; Write-Host '{}'; (Get-Location).Path",
                command, CWD_MARKER
            );
            Command::new("powershell")
                .raw_arg("-NoProfile")
                .raw_arg("-NonInteractive")
                .raw_arg("-Command")
                .raw_arg(&script)
                .current_dir(cwd)
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
        }
        _ => {
            // cmd：chcp 切换 UTF-8，执行用户命令，末尾输出标记和当前目录
            // 注意：不在这里写 cd /D，工作目录通过 current_dir() 设置
            // %CD% 是 cmd 内置变量，始终包含带盘符的完整当前目录，跨盘符也正确
            let cmd_str = format!(
                "chcp 65001 > nul 2>&1 & {} & echo {} & echo %CD%",
                command, CWD_MARKER
            );
            Command::new("cmd")
                .raw_arg("/C")
                .raw_arg(&cmd_str)
                .current_dir(cwd)
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
        }
    }
}

/// 从 stdout 中分离用户输出和捕获到的 cwd
fn extract_cwd_from_output(stdout: &str, fallback_cwd: &str) -> (String, String) {
    if let Some(marker_pos) = stdout.rfind(CWD_MARKER) {
        let user_part = &stdout[..marker_pos];
        let after_marker = &stdout[marker_pos + CWD_MARKER.len()..];
        let new_cwd = after_marker
            .lines()
            .find(|l| !l.trim().is_empty())
            .unwrap_or(fallback_cwd)
            .trim()
            .to_string();
        (user_part.to_string(), new_cwd)
    } else {
        (stdout.to_string(), fallback_cwd.to_string())
    }
}

fn decode_bytes(bytes: &[u8]) -> String {
    if let Ok(s) = std::str::from_utf8(bytes) {
        return s.to_string();
    }
    let (cow, _, _) = encoding_rs::GBK.decode(bytes);
    cow.into_owned()
}

enum WaitResult {
    Finished { exit_code: i32, stdout_bytes: Vec<u8>, stderr_bytes: Vec<u8> },
    Timeout,
    Error(String),
}

fn wait_with_timeout(mut child: std::process::Child, timeout: Duration) -> WaitResult {
    let start = std::time::Instant::now();

    let stdout_handle = child.stdout.take().map(|mut s| {
        thread::spawn(move || { let mut buf = Vec::new(); let _ = s.read_to_end(&mut buf); buf })
    });
    let stderr_handle = child.stderr.take().map(|mut s| {
        thread::spawn(move || { let mut buf = Vec::new(); let _ = s.read_to_end(&mut buf); buf })
    });

    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let stdout_bytes = stdout_handle.and_then(|h| h.join().ok()).unwrap_or_default();
                let stderr_bytes = stderr_handle.and_then(|h| h.join().ok()).unwrap_or_default();
                return WaitResult::Finished {
                    exit_code: status.code().unwrap_or(-1),
                    stdout_bytes,
                    stderr_bytes,
                };
            }
            Ok(None) => {
                if start.elapsed() >= timeout {
                    let _ = child.kill();
                    return WaitResult::Timeout;
                }
                thread::sleep(Duration::from_millis(100));
            }
            Err(e) => return WaitResult::Error(e.to_string()),
        }
    }
}
