use serde_json::Value;
use std::io::Read;
use std::os::windows::process::CommandExt;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

/// 可能是交互式的命令关键词，遇到直接拒绝
pub const INTERACTIVE_COMMANDS: &[&str] = &[
    "vim", "vi", "nano", "emacs", "less", "more", "top", "htop",
    "ssh", "ftp", "telnet", "python", "node", "irb", "pry",
];

/// 禁止执行的危险命令（文件/目录删除类）
/// 包含 cmd 和 PowerShell 的原生命令及常见别名
pub const FORBIDDEN_COMMANDS: &[&str] = &[
    // cmd 删除命令
    "del", "erase", "rd", "rmdir",
    // PowerShell 删除命令及其内置别名
    "remove-item", "ri",
    // PowerShell 中 rm / rmdir / del 均是 Remove-Item 的别名
    "rm",
];

/// 检查命令是否被禁止，返回 Some(错误信息) 或 None
pub fn check_forbidden(command: &str) -> Option<String> {
    let first_token = command.split_whitespace().next().unwrap_or("").to_lowercase();
    let exe = first_token
        .trim_end_matches(".exe")
        .trim_end_matches(".cmd");
    if FORBIDDEN_COMMANDS.contains(&exe) {
        Some(format!(
            "错误：命令 \"{}\" 涉及文件删除操作，已被禁止执行。如需删除文件，请手动操作。",
            exe
        ))
    } else {
        None
    }
}

pub fn run_command(args: Value) -> String {
    let command = match args.get("command").and_then(|v| v.as_str()) {
        Some(c) if !c.trim().is_empty() => c.trim().to_string(),
        _ => return "错误：缺少必填参数 command".to_string(),
    };

    // 检测禁止的删除命令
    if let Some(err) = check_forbidden(&command) {
        return err;
    }

    // 检测明显的交互式命令
    let first_token = command.split_whitespace().next().unwrap_or("").to_lowercase();
    let exe_name = first_token
        .trim_end_matches(".exe")
        .trim_end_matches(".cmd");
    if INTERACTIVE_COMMANDS.contains(&exe_name) {
        return format!(
            "错误：命令 \"{}\" 是交互式程序，不支持执行。请改用非交互模式（如 git log、git diff 等）。",
            exe_name
        );
    }

    let cwd = args.get("cwd").and_then(|v| v.as_str()).map(|s| s.to_string());

    let shell = args
        .get("shell")
        .and_then(|v| v.as_str())
        .unwrap_or("cmd");

    let timeout_secs = args
        .get("timeout_secs")
        .and_then(|v| v.as_u64())
        .unwrap_or(30);

    eprintln!(
        "[DEBUG] run_command: shell={}, cwd={:?}, timeout={}s, cmd={}",
        shell, cwd, timeout_secs, command
    );

    let mut cmd = match shell {
        "powershell" | "ps" => {
            let mut c = Command::new("powershell");
            c.raw_arg("-NoProfile")
                .raw_arg("-NonInteractive")
                .raw_arg("-OutputEncoding")
                .raw_arg("UTF8")
                .raw_arg("-Command")
                .raw_arg(&command);
            c
        }
        _ => {
            let mut c = Command::new("cmd");
            // chcp 65001 切换到 UTF-8 代码页，再执行目标命令
            // 用 raw_arg 绕过 Rust 对引号的自动转义，让 cmd.exe 原样接收命令
            let wrapped = format!("chcp 65001 > nul 2>&1 & {}", command);
            c.raw_arg("/C").raw_arg(&wrapped);
            c
        }
    };

    cmd.stdin(Stdio::null()) // 隔离 MCP 的 stdin，防止交互式程序挂起
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    if let Some(ref dir) = cwd {
        cmd.current_dir(dir);
    }

    let child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => return format!("错误：启动进程失败: {}", e),
    };

    // 解析实际工作目录（用于返回给调用方）
    let actual_cwd = cwd
        .as_deref()
        .and_then(|p| std::fs::canonicalize(p).ok())
        .or_else(|| std::env::current_dir().ok())
        .map(|p| {
            let s = p.to_string_lossy().into_owned();
            s.strip_prefix(r"\\?\").unwrap_or(&s).to_string()
        })
        .unwrap_or_else(|| "(未知)".to_string());

    match wait_with_timeout(child, Duration::from_secs(timeout_secs)) {
        WaitResult::Finished { exit_code, stdout_bytes, stderr_bytes } => {
            let stdout = decode_bytes(&stdout_bytes);
            let stderr = decode_bytes(&stderr_bytes);

            let mut output = String::new();
            output.push_str(&format!("cwd: {}", actual_cwd));
            output.push_str(&format!("\nexit_code: {}", exit_code));
            if !stdout.is_empty() {
                output.push_str("\n\n--- stdout ---\n");
                output.push_str(stdout.trim_end());
            }
            if !stderr.is_empty() {
                output.push_str("\n\n--- stderr ---\n");
                output.push_str(stderr.trim_end());
            }
            if stdout.is_empty() && stderr.is_empty() {
                output.push_str("\n\n(无输出)");
            }
            output
        }
        WaitResult::Timeout => {
            format!("错误：命令执行超时（{}秒），进程已终止", timeout_secs)
        }
        WaitResult::Error(e) => {
            format!("错误：等待进程失败: {}", e)
        }
    }
}

/// 解码字节流：优先 UTF-8，失败则回退 GBK（兼容中文 Windows cmd 残留乱码）
fn decode_bytes(bytes: &[u8]) -> String {
    if let Ok(s) = std::str::from_utf8(bytes) {
        return s.to_string();
    }
    let (cow, _, _) = encoding_rs::GBK.decode(bytes);
    cow.into_owned()
}

enum WaitResult {
    Finished {
        exit_code: i32,
        stdout_bytes: Vec<u8>,
        stderr_bytes: Vec<u8>,
    },
    Timeout,
    Error(String),
}

fn wait_with_timeout(mut child: std::process::Child, timeout: Duration) -> WaitResult {
    let start = std::time::Instant::now();

    // 用独立线程并发读取，防止管道缓冲区满死锁
    let stdout_handle = child.stdout.take().map(|mut s| {
        thread::spawn(move || {
            let mut buf = Vec::new();
            let _ = s.read_to_end(&mut buf);
            buf
        })
    });

    let stderr_handle = child.stderr.take().map(|mut s| {
        thread::spawn(move || {
            let mut buf = Vec::new();
            let _ = s.read_to_end(&mut buf);
            buf
        })
    });

    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let stdout_bytes = stdout_handle.and_then(|h| h.join().ok()).unwrap_or_default();
                let stderr_bytes = stderr_handle.and_then(|h| h.join().ok()).unwrap_or_default();
                let exit_code = status.code().unwrap_or(-1);
                return WaitResult::Finished { exit_code, stdout_bytes, stderr_bytes };
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
