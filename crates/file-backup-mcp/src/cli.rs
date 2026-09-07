use colored::*;
use std::io::{self, Write};
use std::fs;

use crate::utils;

pub fn run() {
    utils::init_terminal();
    println!("=== 文件备份助手 ===");

    let target_path = loop {
        if let Some(p) = utils::get_valid_path() {
            break p;
        }
    };

    let absolute_target = fs::canonicalize(&target_path).expect("获取绝对路径失败");
    let output_pth_str = utils::get_output_path(&target_path);
    let absolute_output = fs::canonicalize(&output_pth_str).expect("获取绝对路径失败");

    println!("\n请选择功能");
    println!("1: 生成目录结构");
    println!("2: 根据文件类型备份");
    println!("3: 扁平化目录");
    print!("请输入编号 (1、2、3): ");
    io::stdout().flush().unwrap();

    let mut choice = String::new();
    io::stdin().read_line(&mut choice).expect("读取选择失败");
    let choice = choice.trim();

    match choice {
        "1" => {
            println!("正在生成 JSON...");
            crate::json_gen::run_generate_json_cli(&absolute_target, &absolute_output);
        }
        "2" => {
            println!("请输入需要过滤的文件后缀，逗号分隔（如 .jpg,.png），直接回车则备份所有文件");
            io::stdout().flush().unwrap();
            let mut ext_input = String::new();
            io::stdin().read_line(&mut ext_input).expect("读取后缀失败");
            let ext_input = ext_input.trim();

            println!("你是要 备份(y) 这些后缀的文件，还是 排除(n) 它们？(默认备份 y):");
            io::stdout().flush().unwrap();
            let mut mode_input = String::new();
            io::stdin().read_line(&mut mode_input).expect("备份选择失败");
            let extract_mode = if mode_input.trim().to_lowercase().starts_with('n') {
                utils::FilterMode::Exclude
            } else {
                utils::FilterMode::Extract
            };

            let ignore_empty = utils::ask_ignore_empty_folder(
                "是否忽略空文件夹（只保留含有目标文件的目录，默认不忽略 n）？",
            );

            let res = crate::file_ops_con::run_copy_files_cli(
                &absolute_target,
                &absolute_output,
                ext_input,
                ignore_empty,
                extract_mode,
            );
            match res {
                Ok(path) => {
                    println!("备份完成。是否需要将输出文件夹内的所有文件提取到根目录（扁平化，默认n）？(y/n)");
                    let mut confirm = String::new();
                    io::stdin().read_line(&mut confirm).unwrap();
                    if confirm.trim().to_lowercase() == "y" {
                        crate::file_manage::flatten_directory_cli(&path);
                    }
                }
                Err(e) => {
                    eprintln!("{} {}", "错误:".red().bold(), e);
                }
            }
        }
        "3" => {
            println!(
                "{}",
                "警告，扁平化会将所有文件提取到顶级目录，破坏文件结构，谨慎使用".red().bold()
            );
            println!("是否继续？(y/n)");
            let mut confirm = String::new();
            io::stdin().read_line(&mut confirm).unwrap();
            if confirm.to_lowercase().trim().starts_with('y') {
                crate::file_manage::flatten_directory_cli(&absolute_target);
            } else {
                println!("{}", "取消扁平化".green().bold());
            }
        }
        _ => {
            println!("无效的选择，无任何处理！")
        }
    }

    println!("\n{}", "----------------------------------".bright_black());
    println!("{}", "任务执行完毕，按任意键退出...");

    let term = console::Term::stdout();
    let _ = term.read_key();
}
