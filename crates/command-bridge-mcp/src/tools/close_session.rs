use serde_json::Value;
use crate::session::store;

pub fn close_session(args: Value) -> String {
    let session_id = match args.get("session_id").and_then(|v| v.as_str()) {
        Some(id) => id.to_string(),
        None => return "错误：缺少必填参数 session_id".to_string(),
    };

    let removed = store().lock().unwrap().remove(&session_id);

    match removed {
        Some(s) => {
            eprintln!("[INFO] close_session: id={} 已关闭", session_id);
            format!("session \"{}\" 已关闭（最终工作目录：{}）", session_id, s.cwd)
        }
        None => format!("警告：session \"{}\" 不存在，可能已被关闭", session_id),
    }
}
