use std::path::Path;
use serde_json::Value;

use crate::file_manage;

pub fn flatten_directory(args: Value) -> String {
    let target_path = match args.get("target_path").and_then(|v| v.as_str()) {
        Some(p) => p.to_string(),
        None => return "参数错误：缺少必填参数 target_path".to_string(),
    };

    let target = Path::new(&target_path);
    if !target.exists() {
        return format!("错误：路径不存在 -> {}", target_path);
    }
    if !target.is_dir() {
        return format!("错误：路径不是一个文件夹 -> {}", target_path);
    }

    match file_manage::flatten_directory(target) {
        Ok(msg) => msg,
        Err(e) => format!("扁平化失败：{}", e),
    }
}
