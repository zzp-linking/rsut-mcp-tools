use regex::Regex;
use walkdir::WalkDir;

use crate::config::Config;

/// path_filter 的过滤模式
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FilterMode {
    Include,
    Exclude,
}

impl FilterMode {
    pub fn from_str(s: &str) -> Self {
        if s == "exclude" { FilterMode::Exclude } else { FilterMode::Include }
    }
}

/// 遍历目录，返回所有符合条件的文本文件路径（绝对路径字符串）
///
/// - `directory`：根目录
/// - `config`：全局配置（ignore patterns）
/// - `path_filter`：可选的路径正则过滤
/// - `path_filter_mode`：include 或 exclude
pub fn walk_text_files(
    directory: &str,
    config: &Config,
    path_filter: Option<&Regex>,
    path_filter_mode: FilterMode,
) -> Vec<String> {
    let mut result = Vec::new();

    let walker = WalkDir::new(directory)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| {
            let name = entry.file_name().to_string_lossy();
            let full = entry.path().to_string_lossy();
            // 检查路径中任意部分是否命中 ignore 规则
            !config.should_ignore(&name) && !config.should_ignore(&full)
        });

    for entry in walker.flatten() {
        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path();
        let path_str = path.to_string_lossy().to_string();

        // path_filter 过滤
        if let Some(re) = path_filter {
            let matched = re.is_match(&path_str);
            match path_filter_mode {
                FilterMode::Include if !matched => continue,
                FilterMode::Exclude if matched => continue,
                _ => {}
            }
        }

        // 二进制文件检测：先按扩展名快速判断
        if is_binary_by_extension(path) {
            continue;
        }

        // 跳过前端打包/压缩产物（整行即整个文件，搜索命中会导致上下文溢出）
        if is_minified_asset(path) {
            continue;
        }

        result.push(path_str);
    }

    result
}

/// 遍历目录，返回所有文件路径（不做文本过滤，用于文件名搜索）
pub fn walk_all_files(directory: &str, config: &Config) -> Vec<String> {
    let mut result = Vec::new();

    let walker = WalkDir::new(directory)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| {
            let name = entry.file_name().to_string_lossy();
            let full = entry.path().to_string_lossy();
            !config.should_ignore(&name) && !config.should_ignore(&full)
        });

    for entry in walker.flatten() {
        if entry.file_type().is_file() {
            result.push(entry.path().to_string_lossy().to_string());
        }
    }

    result
}

/// 通过文件扩展名判断是否为二进制文件（快速路径）
fn is_binary_by_extension(path: &std::path::Path) -> bool {
    let binary_exts = [
        "exe", "dll", "so", "dylib", "lib", "a", "o", "obj",
        "png", "jpg", "jpeg", "gif", "bmp", "ico", "webp", "svg",
        "tiff", "psd", "ai",
        "mp3", "mp4", "wav", "avi", "mkv", "mov", "flv", "wmv",
        "zip", "rar", "7z", "tar", "gz", "bz2", "xz", "zst",
        "pdf", "doc", "docx", "xls", "xlsx", "ppt", "pptx",
        "ttf", "otf", "woff", "woff2", "eot",
        "class", "jar", "war", "ear",
        "pyc", "pyo",
        "wasm", "bin", "dat", "db", "sqlite", "sqlite3",
        "lock",
    ];

    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        if binary_exts.contains(&ext.to_lowercase().as_str()) {
            return true;
        }
    }

    // 无扩展名但文件名类似二进制的（可选跳过）
    false
}

/// 检测文件名是否为前端打包/压缩产物（如 foo.min.js、chunk.bundle.js）
///
/// 这类文件通常整行即为整个文件内容，搜索命中后返回会导致上下文溢出，
/// 应在遍历阶段提前跳过。
pub fn is_minified_asset(path: &std::path::Path) -> bool {
    let name = match path.file_name().and_then(|n| n.to_str()) {
        Some(n) => n.to_lowercase(),
        None => return false,
    };

    // 匹配 *.min.js / *.min.css / *.min.mjs 等
    if name.contains(".min.") {
        return true;
    }

    // 匹配 *.bundle.js / *.chunk.js / *.chunk.css 等
    let bundled_patterns = [".bundle.js", ".bundle.mjs", ".chunk.js", ".chunk.mjs", ".chunk.css"];
    if bundled_patterns.iter().any(|p| name.ends_with(p)) {
        return true;
    }

    false
}

/// 读取文件内容，自动处理 UTF-8 和 GBK 编码
pub fn read_file_text(path: &str) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;

    // 先尝试 UTF-8
    if let Ok(text) = std::str::from_utf8(&bytes) {
        return Some(text.to_string());
    }

    // 快速判断：如果字节流中有太多 0x00，可能是 UTF-16 或真正的二进制，跳过
    let null_count = bytes.iter().filter(|&&b| b == 0).count();
    if null_count > bytes.len() / 20 {
        return None;
    }

    // 尝试 GBK 解码
    let (decoded, _, had_errors) = encoding_rs::GBK.decode(&bytes);
    if had_errors {
        return None;
    }

    Some(decoded.into_owned())
}
