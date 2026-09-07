pub mod close_session;
pub mod open_session;
pub mod run_command;
pub mod run_in_session;

use serde_json::{json, Value};

use close_session::close_session as exec_close_session;
use open_session::open_session as exec_open_session;
use run_command::run_command as exec_run_command;
use run_in_session::run_in_session as exec_run_in_session;

pub fn tools_list() -> Value {
    json!([
        {
            "name": "run_command",
            "description": "在本机执行一条 shell 命令（无状态，cd 不跨调用保留）。禁止删除类命令和交互式程序。",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "要执行的命令。"
                    },
                    "cwd": {
                        "type": "string",
                        "description": "工作目录，须为绝对路径。"
                    },
                    "shell": {
                        "type": "string",
                        "enum": ["cmd", "powershell"],
                        "description": "cmd（默认）或 powershell。"
                    },
                    "timeout_secs": {
                        "type": "integer",
                        "description": "超时秒数，默认 30。"
                    }
                },
                "required": ["command"]
            }
        },
        {
            "name": "open_session",
            "description": "创建有状态 shell 会话，返回 session_id。会话内 cd 会保留。",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "cwd": {
                        "type": "string",
                        "description": "初始工作目录，须为绝对路径。"
                    },
                    "shell": {
                        "type": "string",
                        "enum": ["cmd", "powershell"],
                        "description": "cmd（默认）或 powershell，会话内不可更换。"
                    }
                },
                "required": []
            }
        },
        {
            "name": "run_in_session",
            "description": "在已有会话中执行命令。cd 会保留，环境变量不会。禁止删除类命令和交互式程序。",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "session_id": {
                        "type": "string",
                        "description": "open_session 返回的 session_id。"
                    },
                    "command": {
                        "type": "string",
                        "description": "要执行的命令。"
                    },
                    "timeout_secs": {
                        "type": "integer",
                        "description": "超时秒数，默认 30。"
                    }
                },
                "required": ["session_id", "command"]
            }
        },
        {
            "name": "close_session",
            "description": "关闭会话并释放资源。",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "session_id": {
                        "type": "string",
                        "description": "要关闭的 session_id。"
                    }
                },
                "required": ["session_id"]
            }
        }
    ])
}

pub fn call_tool(tool_name: &str, args: Value) -> String {
    match tool_name {
        "run_command"     => exec_run_command(args),
        "open_session"    => exec_open_session(args),
        "run_in_session"  => exec_run_in_session(args),
        "close_session"   => exec_close_session(args),
        _                 => format!("未知工具: {}", tool_name),
    }
}
