use globset::{Glob, GlobSet, GlobSetBuilder};
use serde::Deserialize;
use serde_json::Value;

use crate::config::Config;
use crate::file_walker::walk_all_files;

#[derive(Deserialize)]
pub struct SearchFilesArgs {
    pub pattern: String,
    pub directory: String,
    pub mode: Option<String>,
    pub is_glob: Option<bool>,
}

pub fn search_files(args_value: Value, config: &Config) -> String {
    let args: SearchFilesArgs = match serde_json::from_value(args_value) {
        Ok(a) => a,
        Err(e) => return format!("参数解析失败: {}", e),
    };

    let mode = args.mode.as_deref().unwrap_or("global");
    let is_glob = args.is_glob.unwrap_or(false);
    let single_mode = mode == "single";

    // 构建匹配器
    let glob_set: Option<GlobSet> = if is_glob {
        let mut builder = GlobSetBuilder::new();
        match Glob::new(&args.pattern) {
            Ok(g) => { builder.add(g); }
            Err(e) => return format!("无效的 glob 模式: {}", e),
        }
        match builder.build() {
            Ok(gs) => Some(gs),
            Err(e) => return format!("构建 glob 匹配器失败: {}", e),
        }
    } else {
        None
    };

    let files = walk_all_files(&args.directory, config);

    if files.is_empty() {
        return "指定目录下未找到任何文件。".to_string();
    }

    let mut matched: Vec<String> = Vec::new();

    for path_str in &files {
        let path = std::path::Path::new(path_str);
        let file_name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n,
            None => continue,
        };

        let is_match = if is_glob {
            // glob 同时匹配文件名和相对路径，提升灵活性
            glob_set.as_ref().map_or(false, |gs| {
                gs.is_match(file_name) || gs.is_match(path)
            })
        } else {
            // 精确匹配文件名（大小写敏感）
            file_name == args.pattern
        };

        if is_match {
            matched.push(path_str.clone());
            if single_mode {
                break;
            }
        }
    }

    if matched.is_empty() {
        return format!("未找到名称匹配 {:?} 的文件。", args.pattern);
    }

    let mut out = format!(
        "文件名搜索 {:?} 结果（共 {} 个文件）：\n",
        args.pattern,
        matched.len()
    );
    for p in &matched {
        out.push_str(&format!("  {}\n", p));
    }
    out
}
