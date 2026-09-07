use rayon::prelude::*;
use regex::Regex;
use serde::Deserialize;
use serde_json::Value;
use std::sync::{Arc, Mutex};
use std::sync::atomic::Ordering;

use crate::config::Config;
use crate::file_walker::{walk_text_files, read_file_text, FilterMode};

#[derive(Deserialize)]
pub struct SearchInFilesArgs {
    pub query: String,
    pub directory: String,
    pub mode: Option<String>,
    pub is_regex: Option<bool>,
    pub case_sensitive: Option<bool>,
    pub path_filter: Option<String>,
    pub path_filter_mode: Option<String>,
    /// 本次调用额外忽略的路径正则，与启动参数 --ignore 合并生效。
    pub ignore: Option<String>,
    pub context_lines: Option<usize>,
    pub max_results: Option<usize>,
    /// 单行输出的最大字符数（默认 300）。超出部分会被截断并附注原始长度。
    /// 主要防止命中压缩/混淆代码时整行内容过大撑爆上下文。
    /// 若需查看完整行内容，可将此值调大后重试。
    pub max_line_chars: Option<usize>,
    /// 整个搜索结果的最大总字符数（默认 8000）。
    /// 超过上限后停止追加并附注提示，防止多文件多匹配累计溢出上下文。
    pub max_total_chars: Option<usize>,
}

pub struct FileMatch {
    pub path: String,
    pub hits: Vec<LineHit>,
}

pub struct LineHit {
    pub line_no: usize,
    pub lines: Vec<(usize, String)>, // (行号, 内容)，包含上下文
}

pub fn search_in_files(args_value: Value, config: &Config) -> String {
    let args: SearchInFilesArgs = match serde_json::from_value(args_value) {
        Ok(a) => a,
        Err(e) => return format!("参数解析失败: {}", e),
    };

    let mode = args.mode.as_deref().unwrap_or("global");
    let is_regex = args.is_regex.unwrap_or(false);
    let case_sensitive = args.case_sensitive.unwrap_or(true);
    let context_lines = args.context_lines.unwrap_or(0);
    let max_results = args.max_results.unwrap_or(50);
    let max_line_chars = args.max_line_chars.unwrap_or(300);
    let max_total_chars = args.max_total_chars.unwrap_or(8000);

    let config = match config.with_call_ignore(args.ignore.as_deref()) {
        Ok(c) => c,
        Err(e) => return e,
    };

    // 构建搜索正则
    let pattern_str = if is_regex {
        args.query.clone()
    } else {
        regex::escape(&args.query)
    };
    let pattern_str = if case_sensitive {
        pattern_str
    } else {
        format!("(?i){}", pattern_str)
    };
    let search_re = match Regex::new(&pattern_str) {
        Ok(re) => re,
        Err(e) => return format!("无效的搜索表达式: {}", e),
    };

    // 构建路径过滤正则
    let path_filter_re: Option<Regex> = if let Some(ref pf) = args.path_filter {
        match Regex::new(pf) {
            Ok(re) => Some(re),
            Err(e) => return format!("无效的路径过滤表达式: {}", e),
        }
    } else {
        None
    };
    let filter_mode = FilterMode::from_str(
        args.path_filter_mode.as_deref().unwrap_or("include"),
    );

    // 遍历文件
    let files = walk_text_files(
        &args.directory,
        &config,
        path_filter_re.as_ref(),
        filter_mode,
    );

    if files.is_empty() {
        return "未找到任何可搜索的文本文件。".to_string();
    }

    let single_mode = mode == "single";
    let results: Arc<Mutex<Vec<FileMatch>>> = Arc::new(Mutex::new(Vec::new()));
    let found_single = Arc::new(std::sync::atomic::AtomicBool::new(false));

    files.par_iter().for_each(|path| {
        // single 模式下已找到，跳过剩余文件
        if single_mode && found_single.load(Ordering::Relaxed) {
            return;
        }

        // 当前已收集结果超限，停止
        {
            let lock = results.lock().unwrap();
            if lock.len() >= max_results {
                return;
            }
        }

        let text = match read_file_text(path) {
            Some(t) => t,
            None => return,
        };

        let all_lines: Vec<&str> = text.lines().collect();
        let mut hits: Vec<LineHit> = Vec::new();

        for (idx, line) in all_lines.iter().enumerate() {
            if !search_re.is_match(line) {
                continue;
            }

            let line_no = idx + 1;
            let start = idx.saturating_sub(context_lines);
            let end = (idx + context_lines + 1).min(all_lines.len());

            let context: Vec<(usize, String)> = all_lines[start..end]
                .iter()
                .enumerate()
                .map(|(i, &l)| (start + i + 1, l.to_string()))
                .collect();

            hits.push(LineHit { line_no, lines: context });

            if single_mode {
                break;
            }
        }

        if !hits.is_empty() {
            let mut lock = results.lock().unwrap();
            if lock.len() < max_results {
                lock.push(FileMatch { path: path.clone(), hits });
                if single_mode {
                    found_single.store(true, Ordering::Relaxed);
                }
            }
        }
    });

    let results = results.lock().unwrap();

    if results.is_empty() {
        return format!("未找到包含 {:?} 的文件。", args.query);
    }

    format_results(&results, &args.query, max_results, max_line_chars, max_total_chars)
}

fn format_results(
    results: &[FileMatch],
    query: &str,
    max_results: usize,
    max_line_chars: usize,
    max_total_chars: usize,
) -> String {
    let mut out = String::new();
    let truncated = results.len() >= max_results;

    out.push_str(&format!(
        "搜索 {:?} 的结果（共 {} 个文件{}）：\n",
        query,
        results.len(),
        if truncated { "，已达上限" } else { "" }
    ));

    // 记录 header 部分长度，正文超限时保留 header
    let header_len = out.len();
    let body_limit = max_total_chars.saturating_sub(header_len);
    let mut body = String::new();
    let mut total_overflow = false;

    'outer: for fm in results {
        let file_header = format!("\n文件: {}\n", fm.path);
        if body.len() + file_header.len() > body_limit {
            total_overflow = true;
            break;
        }
        body.push_str(&file_header);

        let mut last_no: Option<usize> = None;
        for hit in &fm.hits {
            for (no, line) in &hit.lines {
                // 避免上下文行重复打印
                if let Some(last) = last_no {
                    if *no <= last {
                        continue;
                    }
                }
                let marker = if *no == hit.line_no { ">>>" } else { "   " };

                // 单行字符截断
                let char_count = line.chars().count();
                let display_line: std::borrow::Cow<str> = if char_count > max_line_chars {
                    let truncated_str: String = line.chars().take(max_line_chars).collect();
                    format!(
                        "{}...(已截断，原始共 {} 字符，可增大 max_line_chars 查看完整内容)",
                        truncated_str, char_count
                    )
                    .into()
                } else {
                    line.as_str().into()
                };

                let row = format!("  {} {:>5}: {}\n", marker, no, display_line);

                // 总量超限检查
                if body.len() + row.len() > body_limit {
                    total_overflow = true;
                    break 'outer;
                }

                body.push_str(&row);
                last_no = Some(*no);
            }
        }
    }

    out.push_str(&body);

    if total_overflow {
        out.push_str(&format!(
            "\n[输出已达总字符上限 {}，部分结果被省略。可增大 max_total_chars 查看更多内容]\n",
            max_total_chars
        ));
    }

    out
}
