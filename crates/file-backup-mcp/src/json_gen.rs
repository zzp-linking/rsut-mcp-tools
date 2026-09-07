use std::path::Path;
use std::fs;
use std::fs::File;
use std::collections::HashMap;
use serde::Serialize;
use colored::*;
use std::io::Write;
use crate::utils::clean_path_string;

#[derive(Serialize)]
#[serde(untagged)]
enum FileNode {
    File(String),
    Folder(HashMap<String, FileNode>)
}
const INDENT_SYMBOL: &str = "|﹎";

/// 生成目录结构的 JSON 和 TXT 文件，返回执行结果摘要字符串
pub fn run_generate_json(absolute_path: &Path, absolute_output: &Path) -> Result<String, String> {
    let folder_name = absolute_path.file_name().and_then(|n| n.to_str()).unwrap_or("备份");

    let mut json_path = absolute_output.to_path_buf();
    json_path.push(format!("{}.json", folder_name));
    let mut txt_path = absolute_output.to_path_buf();
    txt_path.push(format!("{}.txt", folder_name));

    let mut txt_content = format!("【{}】\n", folder_name);

    let tree = build_tree(absolute_path, 1, &mut txt_content);
    let mut root_map = HashMap::new();
    root_map.insert(folder_name.to_string(), tree);
    let tree = FileNode::Folder(root_map);

    let json_output = serde_json::to_string_pretty(&tree).unwrap();
    let mut json_file = File::create(&json_path).map_err(|e| format!("创建JSON文件失败: {}", e))?;
    json_file.write_all(json_output.as_bytes()).map_err(|e| format!("写入JSON文件失败: {}", e))?;

    let mut txt_file = File::create(&txt_path).map_err(|e| format!("创建TXT文件失败: {}", e))?;
    txt_file.write_all(txt_content.as_bytes()).map_err(|e| format!("写入TXT文件失败: {}", e))?;

    let json_path_str = clean_path_string(&json_path);
    let txt_path_str = clean_path_string(&txt_path);
    Ok(format!(
        "目录结构生成成功！\nJSON 文件: {}\nTXT 文件: {}",
        json_path_str, txt_path_str
    ))
}

/// CLI 包装：调用 run_generate_json 并将结果打印到终端
pub fn run_generate_json_cli(absolute_path: &Path, absolute_output: &Path) {
    match run_generate_json(absolute_path, absolute_output) {
        Ok(msg) => {
            let parts: Vec<&str> = msg.lines().collect();
            if parts.len() >= 3 {
                println!("{} 文件已生成在: {}", "成功!".green().bold(), parts[1].trim_start_matches("JSON 文件: ").cyan());
                println!("{} 文件已生成在: {}", "成功!".green().bold(), parts[2].trim_start_matches("TXT 文件: ").cyan());
            } else {
                println!("{}", msg);
            }
        }
        Err(e) => eprintln!("{} {}", "错误:".red().bold(), e),
    }
}

fn build_tree (path: &Path, depth: usize, txt_output: &mut String) -> FileNode {
    let metadata = fs::metadata(path).expect("读取元数据失败");
    let file_or_folder_name = path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Unkonwn")
        .to_string();

    if metadata.is_file() {
        // 如果是文件，直接返回文件名字符串
        let prefix = INDENT_SYMBOL.repeat(depth);
        txt_output.push_str(&format!("{} {}\n", prefix, file_or_folder_name));
        FileNode::File(String::from("√"))
    } else {
        // 如果是目录，递归读取
        let mut folder_map = HashMap::new();

        if let Ok(entries) = fs::read_dir(path) {
            // 对entry进行排序，保证生成的 TXT 也是有序的（文件夹在前，文件在后）
            let mut entries_vec: Vec<_> = entries.flatten().collect();
            entries_vec.sort_by_key(|e| (!e.path().is_dir(), e.file_name()));

            for entry in entries_vec {
                let entry_path = entry.path();
                let entry_name = entry_path.file_name().and_then(|n| n.to_str()).unwrap_or("Unkonwn");

                // TXT 逻辑，如果是文件夹，先在这里记录一层目录名
                if entry_path.is_dir() { 
                    let prefix = INDENT_SYMBOL.repeat(depth);
                    txt_output.push_str(&format!("{}【{}】\n", prefix, entry_name));
                    // 递归下一层， depth +1
                    folder_map.insert(entry_name.to_string(),build_tree(&entry_path, depth + 1, txt_output));
                } else {
                    // 如果是文件，直接交给下一层递归处理（它会金 is_file 分支）
                    folder_map.insert(entry_name.to_string(), build_tree(&entry_path, depth, txt_output));
                }
            }
        }
        FileNode::Folder(folder_map)
    }
}