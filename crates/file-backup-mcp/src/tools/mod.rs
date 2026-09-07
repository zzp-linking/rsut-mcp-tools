pub mod backup_files;
pub mod flatten_directory;
pub mod generate_directory_structure;

use serde_json::{json, Value};

use backup_files::backup_files;
use flatten_directory::flatten_directory;
use generate_directory_structure::generate_directory_structure;

pub fn tools_list() -> Value {
    json!([
        {
            "name": "generate_directory_structure",
            "description": "扫描文件夹，生成 JSON 结构文件和 TXT 树状预览。",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "source_path": {
                        "type": "string",
                        "description": "源文件夹绝对路径。"
                    },
                    "output_path": {
                        "type": "string",
                        "description": "输出目录绝对路径。不填则保存在源文件夹的父目录。"
                    }
                },
                "required": ["source_path"]
            }
        },
        {
            "name": "backup_files",
            "description": "按正则匹配文件完整路径（正斜杠分隔）并备份。pattern 为空则备份全部。",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "source_path": {
                        "type": "string",
                        "description": "源文件夹绝对路径。"
                    },
                    "output_path": {
                        "type": "string",
                        "description": "输出目录绝对路径。不填则在源文件夹父目录下创建备份目录。"
                    },
                    "pattern": {
                        "type": "string",
                        "description": "匹配文件完整路径的正则，路径以 / 分隔。为空则备份全部。"
                    },
                    "ignore_empty_folders": {
                        "type": "boolean",
                        "description": "是否忽略空文件夹，默认 false。"
                    }
                },
                "required": ["source_path"]
            }
        },
        {
            "name": "flatten_directory",
            "description": "将子目录中的文件全部移到根目录并删除空子目录。不可逆。",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "target_path": {
                        "type": "string",
                        "description": "目标文件夹绝对路径。"
                    }
                },
                "required": ["target_path"]
            }
        }
    ])
}

pub fn call_tool(tool_name: &str, args: Value) -> String {
    match tool_name {
        "generate_directory_structure" => generate_directory_structure(args),
        "backup_files" => backup_files(args),
        "flatten_directory" => flatten_directory(args),
        _ => format!("未知工具: {}", tool_name),
    }
}
