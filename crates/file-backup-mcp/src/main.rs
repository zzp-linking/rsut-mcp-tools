mod cli;
mod file_manage;
mod file_ops_con;
mod json_gen;
mod mcp_server;
mod tools;
mod utils;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let is_mcp = args.iter().skip(1).any(|a| a == "mcp" || a == "--mcp");

    if is_mcp {
        eprintln!("[INFO] file-backup-mcp 启动 (MCP 模式)");
        mcp_server::run_loop();
    } else {
        cli::run();
    }
}
