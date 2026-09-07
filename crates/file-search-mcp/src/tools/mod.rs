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
            "description": "在指定目录下按内容搜索文件，返回匹配路径和行号。",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "要搜索的字符串。is_regex=true 时作为正则处理。"
                    },
                    "directory": {
                        "type": "string",
                        "description": "搜索根目录，须为绝对路径。"
                    },
                    "mode": {
                        "type": "string",
                        "enum": ["single", "global"],
                        "description": "single 找到第一个匹配文件即停止，global 返回全部。默认 global。"
                    },
                    "is_regex": {
                        "type": "boolean",
                        "description": "query 是否为正则，默认 false。"
                    },
                    "case_sensitive": {
                        "type": "boolean",
                        "description": "是否区分大小写，默认 true。"
                    },
                    "path_filter": {
                        "type": "string",
                        "description": "对文件绝对路径的正则过滤，配合 path_filter_mode 使用。"
                    },
                    "path_filter_mode": {
                        "type": "string",
                        "enum": ["include", "exclude"],
                        "description": "include 只搜索匹配项，exclude 跳过匹配项。默认 include。"
                    },
                    "context_lines": {
                        "type": "integer",
                        "description": "匹配行前后各 N 行上下文，默认 0。"
                    },
                    "max_results": {
                        "type": "integer",
                        "description": "最多返回的匹配文件数，默认 50。"
                    },
                    "max_line_chars": {
                        "type": "integer",
                        "description": "单行输出的最大字符数，默认 300。"
                    },
                    "max_total_chars": {
                        "type": "integer",
                        "description": "整个搜索结果的最大总字符数，默认 8000。"
                    },
                    "ignore": {
                        "type": "string",
                        "description": "本次调用额外忽略的路径正则，与启动 --ignore 合并生效。"
                    }
                },
                "required": ["query", "directory"]
            }
        },
        {
            "name": "list_directory",
            "description": "列举指定目录下的文件和子目录，以树形结构返回。",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "directory": {
                        "type": "string",
                        "description": "要列举的目录，须为绝对路径。"
                    },
                    "depth": {
                        "type": "integer",
                        "description": "递归深度，默认 1。-1 表示不限制。"
                    },
                    "show_hidden": {
                        "type": "boolean",
                        "description": "是否显示以 . 开头的隐藏项，默认 false。"
                    },
                    "filter": {
                        "type": "string",
                        "description": "按名称的 glob 过滤。"
                    },
                    "ignore": {
                        "type": "string",
                        "description": "本次调用额外忽略的路径正则，与启动 --ignore 合并生效。"
                    }
                },
                "required": ["directory"]
            }
        },
        {
            "name": "search_files",
            "description": "按文件名搜索，返回匹配文件的完整路径。",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "文件名模式。is_glob=false 为精确匹配，true 为 glob。"
                    },
                    "directory": {
                        "type": "string",
                        "description": "搜索根目录，须为绝对路径。"
                    },
                    "mode": {
                        "type": "string",
                        "enum": ["single", "global"],
                        "description": "single 找到第一个即停止，global 返回全部。默认 global。"
                    },
                    "is_glob": {
                        "type": "boolean",
                        "description": "pattern 是否为 glob，默认 false。"
                    },
                    "ignore": {
                        "type": "string",
                        "description": "本次调用额外忽略的路径正则，与启动 --ignore 合并生效。"
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
