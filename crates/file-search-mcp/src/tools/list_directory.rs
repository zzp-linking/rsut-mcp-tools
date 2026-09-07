use globset::{Glob, GlobMatcher};
use serde::Deserialize;
use serde_json::Value;
use std::path::Path;
use walkdir::WalkDir;

use crate::config::Config;

#[derive(Deserialize)]
pub struct ListDirectoryArgs {
    pub directory: String,
    pub depth: Option<i64>,
    pub show_hidden: Option<bool>,
    pub filter: Option<String>,
}

struct Entry {
    /// 相对于根目录的深度（根目录自身为 0）
    depth: usize,
    name: String,
    is_dir: bool,
    size_bytes: Option<u64>,
}

pub fn list_directory(args_value: Value, config: &Config) -> String {
    let args: ListDirectoryArgs = match serde_json::from_value(args_value) {
        Ok(a) => a,
        Err(e) => return format!("参数解析失败: {}", e),
    };

    let show_hidden = args.show_hidden.unwrap_or(false);
    let depth_param = args.depth.unwrap_or(1);

    // depth <= 0 视为无限递归，max_depth(0) 在 WalkDir 中表示只有根目录自身
    // WalkDir.max_depth 是从根目录计算的层数，1 = 只含直接子项
    let max_depth: usize = if depth_param <= 0 {
        usize::MAX
    } else {
        depth_param as usize
    };

    // 解析 glob 过滤模式
    let glob_pattern: Option<GlobMatcher> = if let Some(ref f) = args.filter {
        match Glob::new(f) {
            Ok(g) => Some(g.compile_matcher()),
            Err(e) => return format!("无效的 filter glob 模式: {}", e),
        }
    } else {
        None
    };

    let root = Path::new(&args.directory);
    if !root.exists() {
        return format!("目录不存在: {}", args.directory);
    }
    if !root.is_dir() {
        return format!("路径不是目录: {}", args.directory);
    }

    let mut entries: Vec<Entry> = Vec::new();
    let mut ignored_count: usize = 0;
    let mut hidden_filtered: usize = 0;

    let walker = WalkDir::new(&args.directory)
        .max_depth(max_depth)
        .follow_links(false)
        .sort_by(|a, b| {
            // 目录排在文件前，同类型按名称排序
            let a_dir = a.file_type().is_dir();
            let b_dir = b.file_type().is_dir();
            match (a_dir, b_dir) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.file_name().cmp(b.file_name()),
            }
        })
        .into_iter();

    for result in walker {
        let entry = match result {
            Ok(e) => e,
            Err(_) => continue,
        };

        let entry_path = entry.path();

        // 跳过根目录自身
        if entry_path == root {
            continue;
        }

        let name = entry.file_name().to_string_lossy().to_string();

        // 隐藏文件过滤（以 . 开头）
        if !show_hidden && name.starts_with('.') {
            hidden_filtered += 1;
            continue;
        }

        // ignore 规则过滤
        let full_str = entry_path.to_string_lossy();
        if config.should_ignore(&name) || config.should_ignore(full_str.as_ref()) {
            ignored_count += 1;
            continue;
        }

        // glob 过滤（对文件名匹配，目录名也参与过滤）
        if let Some(ref pat) = glob_pattern {
            if !pat.is_match(&name) {
                continue;
            }
        }

        let is_dir = entry.file_type().is_dir();
        let size_bytes = if is_dir {
            None
        } else {
            std::fs::metadata(entry_path).ok().map(|m| m.len())
        };

        // depth 是相对根目录的层级（直接子项为 1）
        let rel_depth = entry.depth();

        entries.push(Entry {
            depth: rel_depth,
            name,
            is_dir,
            size_bytes,
        });
    }

    format_output(&args.directory, &entries, ignored_count, hidden_filtered, show_hidden, max_depth)
}

fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.1} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

fn format_output(
    directory: &str,
    entries: &[Entry],
    ignored_count: usize,
    hidden_filtered: usize,
    show_hidden: bool,
    max_depth: usize,
) -> String {
    if entries.is_empty() {
        let mut note = String::new();
        if ignored_count > 0 {
            note.push_str(&format!("（{} 项被 ignore 规则过滤）", ignored_count));
        }
        if hidden_filtered > 0 {
            note.push_str(&format!("（{} 个隐藏项未显示，可设 show_hidden=true 查看）", hidden_filtered));
        }
        return format!("目录为空: {}{}", directory, note);
    }

    let depth_note = if max_depth == usize::MAX {
        "全部层级".to_string()
    } else {
        format!("depth={}", max_depth)
    };

    let mut header_notes: Vec<String> = Vec::new();
    header_notes.push(format!("共 {} 项", entries.len()));
    header_notes.push(depth_note);
    if !show_hidden && hidden_filtered > 0 {
        header_notes.push(format!("{} 个隐藏项已过滤", hidden_filtered));
    }
    if ignored_count > 0 {
        header_notes.push(format!("{} 项被 ignore 规则跳过", ignored_count));
    }

    let mut out = format!("目录: {}  ({})\n", directory, header_notes.join("，"));

    let total = entries.len();
    for (i, entry) in entries.iter().enumerate() {
        let is_last = i == total - 1;

        // 生成缩进前缀：depth=1 无缩进，depth>1 加对应空格
        let indent = if entry.depth > 1 {
            "    ".repeat(entry.depth - 1)
        } else {
            String::new()
        };

        let connector = if is_last { "└── " } else { "├── " };

        let type_tag = if entry.is_dir { "[DIR] " } else { "[FILE]" };

        let name_display = if entry.is_dir {
            format!("{}/", entry.name)
        } else {
            entry.name.clone()
        };

        let size_display = match entry.size_bytes {
            Some(b) => format!("  ({})", format_size(b)),
            None => String::new(),
        };

        out.push_str(&format!(
            "{}{}{} {}{}\n",
            indent, connector, type_tag, name_display, size_display
        ));
    }

    out
}
