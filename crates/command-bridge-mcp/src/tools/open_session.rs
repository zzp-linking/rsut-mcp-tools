use serde_json::Value;
use crate::session::{Session, generate_id, store};

pub fn open_session(args: Value) -> String {
    let shell = args
        .get("shell")
        .and_then(|v| v.as_str())
        .unwrap_or("cmd")
        .to_string();

    if shell != "cmd" && shell != "powershell" {
        return format!("错误：不支持的 shell 类型 \"{}\"，可选值：cmd、powershell", shell);
    }

    // 初始工作目录：优先用传入的 cwd，否则用进程当前目录
    let cwd = match args.get("cwd").and_then(|v| v.as_str()) {
        Some(p) => p.to_string(),
        None => std::env::current_dir()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|_| "C:\\".to_string()),
    };

    // 验证目录存在
    if !std::path::Path::new(&cwd).is_dir() {
        return format!("错误：目录不存在或不是文件夹: {}", cwd);
    }

    // 规范化路径（解析 . / .. / 软链接），并去掉 Windows 扩展路径前缀 \\?\
    let cwd = std::fs::canonicalize(&cwd)
        .map(|p| {
            let s = p.to_string_lossy().into_owned();
            s.strip_prefix(r"\\?\").unwrap_or(&s).to_string()
        })
        .unwrap_or(cwd);

    let session_id = generate_id();
    store().lock().unwrap().insert(
        session_id.clone(),
        Session { cwd: cwd.clone(), shell: shell.clone() },
    );

    eprintln!("[INFO] open_session: id={}, shell={}, cwd={}", session_id, shell, cwd);

    format!(
        "session_id: {}\ncwd: {}\nshell: {}\n\n提示：使用 run_in_session 执行命令（cd 的跳转会在本 session 内持久保留），完成所有操作后请调用 close_session 释放资源。",
        session_id, cwd, shell
    )
}
