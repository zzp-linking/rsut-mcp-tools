use std::fs;
use std::path::Path;
use serde_json::Value;

use crate::file_ops_con;
use crate::utils;

pub fn backup_files(args: Value) -> String {
    let source_path = match args.get("source_path").and_then(|v| v.as_str()) {
        Some(p) => p.to_string(),
        None => return "参数错误：缺少必填参数 source_path".to_string(),
    };
    let pattern = args
        .get("pattern")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let ignore_empty = args
        .get("ignore_empty_folders")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

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

    match file_ops_con::run_copy_files(
        &absolute_source,
        &absolute_output,
        &pattern,
        false,
        ignore_empty,
        false,
    ) {
        Ok(result) => {
            let output_path_display = utils::clean_path_string(&result.output_path);
            let pattern_desc = if pattern.is_empty() {
                "无（全量备份）".to_string()
            } else {
                format!("`{}`", pattern)
            };
            let mut summary = format!(
                "备份完成！\n匹配规则：{}\n输出目录：{}\n成功备份：{} 个文件\n备份失败：{} 个文件",
                pattern_desc, output_path_display, result.success, result.fail
            );
            if !result.error_logs.is_empty() {
                summary.push_str("\n\n失败详情：");
                for log in &result.error_logs {
                    summary.push('\n');
                    summary.push_str(log);
                }
            }
            summary
        }
        Err(e) => format!("备份失败：{}", e),
    }
}
