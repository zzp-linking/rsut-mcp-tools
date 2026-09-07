mod mcp_server;
mod session;
mod tools;

fn main() {
    eprintln!("[INFO] command-bridge-mcp 启动");
    mcp_server::run_loop();
}
