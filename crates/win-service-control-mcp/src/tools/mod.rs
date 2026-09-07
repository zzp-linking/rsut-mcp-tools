pub mod manage_processes;
pub mod manage_services;

use serde_json::{json, Value};

use crate::process_handler::ProcessManager;
use crate::service_handler::ServiceManager;
use manage_processes::manage_processes;
use manage_services::manage_services;

pub fn tools_list() -> Value {
    json!([
        {
            "name": "manage_processes",
            "description": "查询或终止 Windows 进程。",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["list", "kill"],
                        "description": "list 查询进程，kill 终止进程。"
                    },
                    "processes": {
                        "type": "string",
                        "description": "进程名或 PID，逗号分隔。list 可省略，kill 必填。"
                    }
                },
                "required": ["action"]
            }
        },
        {
            "name": "manage_services",
            "description": "查询、启动或停止 Windows 服务。",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["list", "open", "stop"],
                        "description": "list 查询，open 启动，stop 停止。"
                    },
                    "services": {
                        "type": "string",
                        "description": "服务名，逗号分隔。list 时可作过滤关键字。"
                    },
                    "only_running": {
                        "type": "boolean",
                        "description": "list 时是否只显示运行中的服务，默认 true。"
                    },
                    "permanent": {
                        "type": "boolean",
                        "description": "open 时设为自动启动，stop 时设为禁用。"
                    },
                    "manual": {
                        "type": "boolean",
                        "description": "open/stop 时将启动类型设为手动。"
                    }
                },
                "required": ["action"]
            }
        }
    ])
}

pub fn call_tool(
    tool_name: &str,
    args: Value,
    manager: &ServiceManager,
    proc_manager: &ProcessManager,
) -> String {
    match tool_name {
        "manage_services" => manage_services(args, manager),
        "manage_processes" => manage_processes(args, proc_manager),
        _ => format!("未知工具: {}", tool_name),
    }
}
