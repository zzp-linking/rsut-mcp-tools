mod config;
mod file_walker;
mod mcp_server;
mod tools;

use config::Config;

fn main() {
    let config = Config::from_args();
    eprintln!(
        "[INFO] file-search-mcp 启动，已加载 {} 条忽略规则",
        config.ignore_patterns.len()
    );
    mcp_server::run_loop(config);
}
