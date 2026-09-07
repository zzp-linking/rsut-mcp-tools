// 并发文件操作
use std::path::{Path, PathBuf};
use std::fs;
use colored::*;
use rayon::prelude::*;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicUsize, Ordering};
use indicatif::{ProgressBar, ProgressStyle};
use regex::Regex;
use crate::utils;

pub struct CopyResult {
    pub output_path: PathBuf,
    pub success: usize,
    pub fail: usize,
    pub error_logs: Vec<String>,
}

/// 核心备份逻辑，返回结构化结果（CLI 和 MCP 共用）
///
/// # 参数
/// - `pattern_str`: 正则表达式字符串，对文件完整路径进行匹配。空字符串表示匹配所有文件。
/// - `invert`: 是否对匹配结果取反。false = 白名单（备份匹配的），true = 黑名单（备份不匹配的）。
/// - `show_progress`: 是否显示终端进度条（CLI 传 true，MCP 传 false）
pub fn run_copy_files(
    path: &Path,
    output: &Path,
    pattern_str: &str,
    invert: bool,
    ingore_empty: bool,
    show_progress: bool,
) -> Result<CopyResult, String> {
    if path == output {
        return Err("源目录和输出目录相同，无法备份".to_string());
    }

    // 编译正则，空字符串视为匹配所有
    let pattern: Option<Regex> = if pattern_str.trim().is_empty() {
        None
    } else {
        Some(Regex::new(pattern_str).map_err(|e| format!("正则表达式无效: {}", e))?)
    };

    let folder_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("copy");
    let mut copy_target_path = output.to_path_buf();
    copy_target_path.push(format!("{}_备份", folder_name));

    if let Err(e) = fs::create_dir_all(&copy_target_path) {
        return Err(format!("无法创建备份根目录: {}", e));
    }

    let mut tasks: Vec<(PathBuf, PathBuf)> = Vec::new();
    collect_copy_task(path, &copy_target_path, pattern.as_ref(), invert, ingore_empty, &mut tasks);

    let total_files = tasks.len();
    if total_files == 0 {
        return Err("没有找到符合条件的文件".to_string());
    }

    let success_count = AtomicUsize::new(0);
    let fail_count = AtomicUsize::new(0);
    let error_logs = Arc::new(Mutex::new(Vec::<String>::new()));

    let pb = if show_progress {
        let bar = ProgressBar::new(total_files as u64);
        bar.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta}) {msg}")
                .unwrap()
                .progress_chars("#>-"),
        );
        Some(bar)
    } else {
        None
    };

    tasks.par_iter().for_each(|(src, dst)| {
        if let Some(ref bar) = pb {
            if let Some(name) = src.file_name() {
                bar.set_message(format!("正在备份: {:?}", name));
            }
        }
        if ingore_empty {
            if let Some(parent) = dst.parent() {
                let _ = fs::create_dir_all(parent);
            }
        }
        match fs::copy(src, dst) {
            Ok(_) => { success_count.fetch_add(1, Ordering::Relaxed); }
            Err(e) => {
                fail_count.fetch_add(1, Ordering::Relaxed);
                let mut logs = error_logs.lock().unwrap();
                logs.push(format!("备份失败：{:?} -> {}", src.file_name().unwrap_or_default(), e));
            }
        }
        if let Some(ref bar) = pb {
            bar.inc(1);
        }
    });

    if let Some(bar) = pb {
        bar.finish_with_message("所有备份任务已完成！");
    }

    let success = success_count.load(Ordering::Relaxed);
    let fail = fail_count.load(Ordering::Relaxed);
    let logs = error_logs.lock().unwrap().clone();

    Ok(CopyResult {
        output_path: copy_target_path,
        success,
        fail,
        error_logs: logs,
    })
}

/// CLI 包装：把后缀列表转为正则，开启进度条，打印结果，返回输出路径
pub fn run_copy_files_cli(
    path: &Path,
    output: &Path,
    ext_str: &str,
    ingore_empty: bool,
    filter_mode: utils::FilterMode,
) -> Result<PathBuf, String> {
    let pattern_str = utils::extensions_to_regex(ext_str).unwrap_or_default();
    // 黑名单模式：正则匹配到了反而不备份，对匹配结果取反即可
    let invert = matches!(filter_mode, utils::FilterMode::Exclude);

    println!("{}", "正在扫描目录并提取任务...".cyan());
    let result = run_copy_files(path, output, &pattern_str, invert, ingore_empty, true)?;

    println!("\n{}", "==============================".bright_black());
    println!("✅ 成功备份: {}", result.success.to_string().green());
    println!("❌ 备份失败: {}", result.fail.to_string().red());
    if !result.error_logs.is_empty() {
        println!("{}", "备份失败清单：".red());
        for log in &result.error_logs {
            println!("{}", log.red());
        }
    }
    println!("{}", "==============================".bright_black());
    Ok(result.output_path)
}

/// 递归扫描源目录，根据正则模式收集备份任务。
///
/// # 参数
/// - `pattern`: 编译好的正则，对文件完整路径进行匹配。`None` 表示匹配所有文件。
/// - `invert`: 是否对匹配结果取反（黑名单模式）。
fn collect_copy_task(
    src: &Path,
    dst: &Path,
    pattern: Option<&Regex>,
    invert: bool,
    ingore_empty: bool,
    tasks: &mut Vec<(PathBuf, PathBuf)>,
) {
    // 防止源目录和目标目录相同导致无限递归
    if let (Ok(s), Ok(d)) = (src.canonicalize(), dst.canonicalize()) {
        if s == d { return; }
    }

    let entries = match fs::read_dir(src) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries {
        if let Ok(entry) = entry {
            let path = entry.path();
            let file_name = path.file_name().unwrap_or_default();
            let target_path = dst.join(file_name);

            if path.is_dir() {
                if !ingore_empty {
                    let _ = fs::create_dir_all(&target_path);
                }
                collect_copy_task(&path, &target_path, pattern, invert, ingore_empty, tasks);
            } else {
                let should_copy = match pattern {
                    // 无正则：全量备份
                    None => true,
                    Some(re) => {
                        // 用完整路径字符串匹配，统一转正斜杠便于跨平台正则书写
                        let matched = re.is_match(&path.to_string_lossy().replace('\\', "/"));
                        if invert { !matched } else { matched }
                    }
                };
                if should_copy {
                    tasks.push((path, target_path));
                }
            }
        }
    }
}
