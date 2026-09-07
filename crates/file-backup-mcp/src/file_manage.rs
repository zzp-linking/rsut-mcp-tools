use std::fs;
use std::path::{Path, PathBuf};
use colored::*;

/// 扁平化核心函数：将 root 内部的所有文件提到 root 根部，并清理 root 内部子文件夹
/// 返回执行结果摘要字符串（CLI 和 MCP 共用）
pub fn flatten_directory(root: &Path) -> Result<String, String> {
    let root_abs = fs::canonicalize(root)
        .map(|p| {
            let s = p.to_string_lossy();
            if s.starts_with(r"\\?\") { PathBuf::from(&s[4..]) } else { p }
        })
        .unwrap_or_else(|_| root.to_path_buf());

    let mut all_files = Vec::new();
    collect_all_files(&root_abs, &mut all_files);

    if all_files.is_empty() {
        return Ok("文件夹为空，无需扁平化。".to_string());
    }

    let total = all_files.len();
    let mut moved = 0usize;

    for src_path in all_files {
        if src_path.parent() == Some(&root_abs) { continue; }

        let file_name = src_path.file_name().unwrap();
        let mut target_path = root_abs.join(file_name);

        if target_path.exists() {
            target_path = generate_unique_path(&target_path);
        }

        if fs::rename(&src_path, &target_path).is_ok() {
            moved += 1;
        }
    }

    if let Ok(entries) = fs::read_dir(&root_abs) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                recursive_delete_dir_force(&path);
            }
        }
    }

    Ok(format!("扁平化完成！共 {} 个文件，成功移动 {} 个到根目录。", total, moved))
}

/// CLI 包装：调用 flatten_directory 并将结果打印到终端
pub fn flatten_directory_cli(root: &Path) {
    match flatten_directory(root) {
        Ok(msg) => println!("{}", msg.green().bold()),
        Err(e) => eprintln!("{} {}", "错误:".red().bold(), e),
    }
}

fn recursive_delete_dir_force(dir: &Path) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                recursive_delete_dir_force(&path);
            }
        }
    }
    let _ = fs::remove_dir(dir); // 删除文件夹本身
}

fn generate_unique_path(path: &Path) -> PathBuf {
    let parent = path.parent().unwrap();
    let stem = path.file_stem().unwrap_or_default().to_string_lossy();
    let ext = path.extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default();
    let mut i = 1;
    loop {
        let new_path = parent.join(format!("{}({}){}", stem, i, ext));
        if !new_path.exists() { return new_path; }
        i += 1;
    }
}

fn collect_all_files(dir: &Path, files: &mut Vec<PathBuf>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() { collect_all_files(&path, files); }
            else { files.push(path); }
        }
    }
}