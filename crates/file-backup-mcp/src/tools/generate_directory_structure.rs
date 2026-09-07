use std::fs;
use std::path::Path;
use serde_json::Value;

use crate::json_gen;

pub fn generate_directory_structure(args: Value) -> String {
    let source_path = match args.get("source_path").and_then(|v| v.as_str()) {
        Some(p) => p.to_string(),
        None => return "参数错误：缺少必填参数 source_path".to_string(),
    };

    let source = Path::new(&source_path);
    if !source.exists() {
        return format!("错误：路径不存在 -> {}", source_path);
    }
    if !source.is_dir() {
        return format!("错误：路径不是一个文件夹 -> {}", source_path);
    }

    let output_path_str = match args.get("output_path").and_then(|v| v.as_str()) {
        Some(p) => {
            if let Err(e) = fs::create_dir_all(p) {
                return format!("错误：无法创建输出目录 {} -> {}", p, e);
            }
            p.to_string()
        }
        None => match Path::new(&source_path).parent() {
            Some(parent) => parent.to_string_lossy().to_string(),
            None => source_path.clone(),
        },
    };

    let absolute_source = match fs::canonicalize(source) {
        Ok(p) => p,
        Err(e) => return format!("错误：无法解析源路径 -> {}", e),
    };
    let absolute_output = match fs::canonicalize(&output_path_str) {
        Ok(p) => p,
        Err(e) => return format!("错误：无法解析输出路径 -> {}", e),
    };

    match json_gen::run_generate_json(&absolute_source, &absolute_output) {
        Ok(msg) => msg,
        Err(e) => format!("执行失败：{}", e),
    }
}
