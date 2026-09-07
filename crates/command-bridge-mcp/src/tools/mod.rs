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
        // ── 无状态单次执行 ─────────────────────────────────────────────
        {
            "name": "run_command",
            "description": "【无状态】在本机执行一条 shell 命令，返回实际工作目录、stdout、stderr 和退出码。\
                \n\n适用场景：单次命令、不需要切换目录。\
                \n\n重要限制：\
                \n- 每次调用都是独立的新进程，cd 命令的效果不会保留到下一次调用。如需执行多条依赖目录状态的命令，请改用 open_session + run_in_session。\
                \n- 禁止执行文件/目录删除命令（del、erase、rd、rmdir、rm、Remove-Item 等），调用会被直接拒绝。\
                \n- 禁止执行交互式程序（vim、ssh 等）。\
                \n\n强烈建议每次都通过 cwd 参数指定工作目录，MCP 进程的默认工作目录通常不是项目目录。",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "要执行的命令，例如 \"git status\" 或 \"dir /B\"。"
                    },
                    "cwd": {
                        "type": "string",
                        "description": "命令的工作目录（绝对路径），例如 \"D:\\my-project\"。强烈建议每次调用都显式传入。"
                    },
                    "shell": {
                        "type": "string",
                        "enum": ["cmd", "powershell"],
                        "description": "使用的 Shell。cmd（默认）或 powershell。"
                    },
                    "timeout_secs": {
                        "type": "integer",
                        "description": "超时秒数，默认 30，超时后自动终止进程。"
                    }
                },
                "required": ["command"]
            }
        },

        // ── Session 模式：有状态多步执行 ──────────────────────────────
        {
            "name": "open_session",
            "description": "【Session 模式 - 第1步】创建一个有状态的 shell 会话，返回 session_id。\
                \n\n在 Session 模式下，cd 命令的结果会持久保留，下一次 run_in_session 调用仍处于切换后的目录。这解决了无状态模式下目录无法跨调用保持的问题。\
                \n\n使用流程：\
                \n  1. open_session → 获得 session_id\
                \n  2. run_in_session（可多次）→ 命令在持久目录下执行\
                \n  3. close_session → 释放资源\
                \n\n适用场景：git 提交流程、多步构建、需要先 cd 进目录再执行一系列操作等。",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "cwd": {
                        "type": "string",
                        "description": "Session 的初始工作目录（绝对路径）。建议传入项目根目录，如 \"D:\\my-project\"。不填则使用 MCP 进程的当前目录（通常不是项目目录）。"
                    },
                    "shell": {
                        "type": "string",
                        "enum": ["cmd", "powershell"],
                        "description": "Session 使用的 Shell 类型，默认 cmd。整个 Session 生命周期内固定，不可中途切换。"
                    }
                },
                "required": []
            }
        },
        {
            "name": "run_in_session",
            "description": "【Session 模式 - 第2步】在已有的 Session 中执行命令。\
                \n\n与 run_command 的关键区别：cd 命令的跳转结果会被记住，下次调用仍处于新目录。返回值包含命令执行后的最新工作目录（cwd 字段），请以此为准判断当前所在位置。\
                \n\n限制：\
                \n- 禁止执行文件/目录删除命令（del、erase、rd、rmdir、rm、Remove-Item 等），调用会被直接拒绝。\
                \n- 禁止执行交互式程序（vim、ssh 等）。\
                \n- 环境变量（set/export）的变更不会跨调用保留，只有工作目录（cd）会持久。",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "session_id": {
                        "type": "string",
                        "description": "由 open_session 返回的 session_id。"
                    },
                    "command": {
                        "type": "string",
                        "description": "要执行的命令。cd 的跳转效果会在本 Session 内持久保留。"
                    },
                    "timeout_secs": {
                        "type": "integer",
                        "description": "超时秒数，默认 30，超时后自动终止进程。"
                    }
                },
                "required": ["session_id", "command"]
            }
        },
        {
            "name": "close_session",
            "description": "【Session 模式 - 第3步】关闭 Session 并释放内存。\
                \n\n完成所有操作后必须调用，否则 Session 数据会一直驻留在内存中。Session 关闭后不可再使用，如需继续执行命令请重新调用 open_session。",
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
