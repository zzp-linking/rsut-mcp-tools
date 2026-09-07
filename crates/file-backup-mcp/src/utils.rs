use std::fs;
use std::path::Path;
use std::io::{self, Write};
use colored::*;
use regex;

#[derive(Debug, Clone, Copy)]
pub enum FilterMode {
    Extract, // 提取模式（白名单）
    Exclude, // 排除模式（黑名单）
}

pub fn init_terminal () {
    // 这一行代码会通过 Windows API 告诉终端：“请进入现代模式”
    // 它会自动处理双击打开的 CMD 窗口
    #[cfg(windows)]
    console::set_colors_enabled(true);
}

pub fn get_valid_path () -> Option<String> {

    // 获取基础路径
    println!("请输入要备份的文件夹路径：");
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).ok();
    let path_str = input.trim();

    if path_str.is_empty() {
        println!("{}", "路径不能为空，请重新输入".red().bold());
        return None;
    }

    let path = Path::new(path_str);
    if !path.exists() {
        println!("{} 路径 '{}' 不存在", "错误".red().bold(), path_str.yellow());
        return None;
    }
    if !path.is_dir() {
        println!("{} 路径 '{}' 不是一个目录", "错误".red().bold(), path_str.yellow());
        return None;
    }
    Some(path_str.to_string())
}

// 获取可选的输出路径
pub fn get_output_path (default_path: &str) -> String {
    println!("请输入结果保存文件夹 (直接回车保存在同层级目录)");
    println!("{}", "注意，输出目录不能在备份文件夹内！！！".red().bold());
    println!("请输入 : ");
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).ok();
    let input = input.trim();

    if input.is_empty() {
        let path = Path::new(default_path);
        match path.parent() {
            Some(parent) => parent.display().to_string(),
            None => default_path.to_string(), // 根目录
        }
    } else {
        // 如果用户输入了路径，尝试创建它（如果不存在）
        if let Err(e) = fs::create_dir_all(input) {
            println!("{} 无法创建目录 {}: {}", "错误".red().bold(), input, e);
            println!("将使用原路径保存在: {}", default_path);
            return default_path.to_string();
        } else {
            input.to_string()
        }
    }
}

// 清理 Windows 的 UNC 路径前缀
pub fn clean_path_string(path: &Path) -> String {
    path.display().to_string().replace(r"\\?\", "")
}

// 询问用户是否忽略空文件夹
pub fn ask_ignore_empty_folder (prommpt: &str) -> bool {
    println!("{} (y/n)", prommpt);
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).ok();
    let input = input.trim().to_lowercase();

    input == "y" || input == "yes"
}

/// 将 CLI 输入的后缀列表（如 ".jpg,.png" 或 "jpg,png"）转为正则表达式字符串。
/// 生成的正则匹配文件完整路径，忽略大小写。
/// 若输入为空，返回 None 表示匹配所有文件。
///
/// 示例：".jpg,.png" -> Some("(?i)\\.(jpg|png)$")
pub fn extensions_to_regex(input: &str) -> Option<String> {
    let exts: Vec<String> = input
        .split(',')
        .map(|s| {
            let s = s.trim().to_lowercase();
            // 去掉开头的点，统一处理
            s.trim_start_matches('.').to_string()
        })
        .filter(|s| !s.is_empty())
        .collect();

    if exts.is_empty() {
        return None;
    }

    // 对每个后缀中的特殊字符进行转义（防止用户输入 c++ 之类的）
    let escaped: Vec<String> = exts
        .iter()
        .map(|e| regex::escape(e))
        .collect();

    Some(format!("(?i)\\.({})$", escaped.join("|")))
}