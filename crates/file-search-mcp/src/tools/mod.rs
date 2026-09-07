pub mod list_directory;
pub mod search_content;
pub mod search_filename;

use serde_json::{json, Value};

use crate::config::Config;
use list_directory::list_directory;
use search_content::search_in_files;
use search_filename::search_files;

/// 返回工具列表的 JSON（用于 tools/list 响应）
pub fn tools_list() -> Value {
    json!([
        {
            "name": "search_in_files",
            "description": "在指定目录下搜索包含特定字符串或正则表达式的文件，返回匹配的文件路径和行号。适用于：查找接口地址、变量名、函数名等代码内容在哪些文件中出现。支持单次匹配（找到第一个即停止）和全局匹配（返回所有结果）。",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "要搜索的字符串。当 is_regex=true 时作为正则表达式处理。"
                    },
                    "directory": {
                        "type": "string",
                        "description": "搜索的根目录，使用绝对路径，例如 D:\\my-project 或 /home/user/project。"
                    },
                    "mode": {
                        "type": "string",
                        "enum": ["single", "global"],
                        "description": "匹配模式。single：找到第一个匹配文件即停止，速度更快；global：搜索所有文件并返回全部结果。默认 global。"
                    },
                    "is_regex": {
                        "type": "boolean",
                        "description": "query 是否为正则表达式，默认 false（普通字符串匹配）。"
                    },
                    "case_sensitive": {
                        "type": "boolean",
                        "description": "是否区分大小写，默认 true。设为 false 可进行大小写不敏感搜索。"
                    },
                    "path_filter": {
                        "type": "string",
                        "description": "对文件的绝对路径做正则过滤，配合 path_filter_mode 使用。例如 \"\\.rs$\" 只搜索 Rust 文件。"
                    },
                    "path_filter_mode": {
                        "type": "string",
                        "enum": ["include", "exclude"],
                        "description": "路径过滤模式。include：只搜索路径匹配 path_filter 的文件；exclude：跳过路径匹配 path_filter 的文件。默认 include。"
                    },
                    "context_lines": {
                        "type": "integer",
                        "description": "返回匹配行前后各 N 行的上下文，默认 0（只返回匹配行本身）。建议设为 2-5 以获得更好的上下文。"
                    },
                    "max_results": {
                        "type": "integer",
                        "description": "最多返回的匹配文件数量，默认 50。防止结果过多超出上下文。"
                    },
                    "max_line_chars": {
                        "type": "integer",
                        "description": "单行输出的最大字符数，默认 300。超出部分会被截断并附注原始字符数。主要防止命中压缩/混淆代码（如打包后的 JS）时整行内容撑爆上下文。若需查看完整行内容，可将此值调大后重试，例如 2000。"
                    },
                    "max_total_chars": {
                        "type": "integer",
                        "description": "整个搜索结果的最大总字符数，默认 8000。超过上限后停止追加并附提示，防止多文件多匹配累计溢出上下文。若结果被截断但仍需更多内容，可将此值调大后重试。"
                    }
                },
                "required": ["query", "directory"]
            }
        },
        {
            "name": "list_directory",
            "description": "列举指定目录下的文件和子目录，以树形结构返回。适用于：了解项目结构、查看某个目录下有哪些文件/子目录。支持控制递归深度、过滤隐藏文件、按 glob 模式筛选名称。",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "directory": {
                        "type": "string",
                        "description": "要列举的目录，使用绝对路径，例如 D:\\my-project 或 /home/user/project。"
                    },
                    "depth": {
                        "type": "integer",
                        "description": "递归深度，默认 1（只列直接子项）。设为 2 或更大可展开更多层级；设为 -1 表示无限递归列出全部内容（大型目录谨慎使用）。"
                    },
                    "show_hidden": {
                        "type": "boolean",
                        "description": "是否显示以 . 开头的隐藏文件/目录，默认 false。"
                    },
                    "filter": {
                        "type": "string",
                        "description": "按名称的 glob 模式过滤，例如 *.rs 只显示 Rust 文件，src* 只显示名称以 src 开头的条目。"
                    }
                },
                "required": ["directory"]
            }
        },
        {
            "name": "search_files",
            "description": "在指定目录下按文件名搜索文件，返回匹配文件的完整路径。不关心文件内容，只按名称查找。支持精确匹配（如 main.rs）和 glob 模式匹配（如 *.rs、*.{ts,tsx}）。",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "文件名模式。is_glob=false 时为精确文件名（如 main.rs、package.json）；is_glob=true 时为 glob 模式（如 *.rs、*.{ts,tsx}、config*）。"
                    },
                    "directory": {
                        "type": "string",
                        "description": "搜索的根目录，使用绝对路径，例如 D:\\my-project 或 /home/user/project。"
                    },
                    "mode": {
                        "type": "string",
                        "enum": ["single", "global"],
                        "description": "匹配模式。single：找到第一个匹配文件即停止；global：返回所有匹配文件。默认 global。"
                    },
                    "is_glob": {
                        "type": "boolean",
                        "description": "pattern 是否为 glob 模式，默认 false（精确文件名匹配）。"
                    }
                },
                "required": ["pattern", "directory"]
            }
        }
    ])
}

/// 执行工具调用，返回结果文本
pub fn call_tool(tool_name: &str, args: Value, config: &Config) -> String {
    match tool_name {
        "list_directory" => list_directory(args, config),
        "search_in_files" => search_in_files(args, config),
        "search_files" => search_files(args, config),
        _ => format!("未知工具: {}", tool_name),
    }
}
