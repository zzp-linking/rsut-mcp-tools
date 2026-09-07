use std::io::{self, BufRead, Write};
use serde_json::{json, Value};

use crate::tools;

pub fn run_loop() {
    let stdin = io::stdin();
    let stdout = io::stdout();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                eprintln!("[ERROR] 读取 stdin 失败: {}", e);
                break;
            }
        };

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let request: Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("[WARN] JSON 解析失败: {}", e);
                continue;
            }
        };

        let id = request.get("id").cloned().unwrap_or(Value::Null);
        let method = request.get("method").and_then(|v| v.as_str()).unwrap_or("");
        let params = request.get("params").cloned();

        eprintln!("[DEBUG] 收到方法: {}", method);

        let response = handle(method, params, id);

        if let Some(resp) = response {
            let mut out = stdout.lock();
            if let Err(e) = writeln!(out, "{}", serde_json::to_string(&resp).unwrap()) {
                eprintln!("[ERROR] 写入 stdout 失败: {}", e);
                break;
            }
        }
    }
}

fn handle(method: &str, params: Option<Value>, id: Value) -> Option<Value> {
    match method {
        "initialize" => Some(json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "protocolVersion": "2024-11-05",
                "capabilities": { "tools": {} },
                "serverInfo": {
                    "name": "command-bridge-mcp",
                    "version": "0.1.0"
                }
            }
        })),

        "notifications/initialized" => None,

        "tools/list" => Some(json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "tools": tools::tools_list()
            }
        })),

        "tools/call" => {
            let params = params.unwrap_or(Value::Null);
            let tool_name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let args = params.get("arguments").cloned().unwrap_or(Value::Null);

            eprintln!("[DEBUG] 调用工具: {}", tool_name);

            let result = tools::call_tool(tool_name, args);

            Some(json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "content": [{ "type": "text", "text": result }]
                }
            }))
        }

        _ => {
            if id == Value::Null {
                None
            } else {
                Some(json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": {
                        "code": -32601,
                        "message": format!("Method not found: {}", method)
                    }
                }))
            }
        }
    }
}
