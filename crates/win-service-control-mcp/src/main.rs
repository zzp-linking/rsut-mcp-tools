mod logger;
mod mcp_server;
mod process_handler;
mod service_handler;
mod tools;

use clap::{Parser, Subcommand};
use process_handler::ProcessManager;
use service_handler::ServiceManager;

#[cfg(not(windows))]
compile_error!("win-service-control-mcp 仅支持 Windows");

#[derive(Parser)]
#[command(name = "wsm", about = "Windows 服务批量管理工具")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 列出所有服务状态，支持关键字过滤 (例如: wsm list vm)
    List { filter: Option<String> },
    /// 列出【所有】状态的服务，支持过滤
    ListAll { filter: Option<String> },
    /// 开启服务
    Open {
        services: Vec<String>,
        /// 设为自动启动 (Permanent)
        #[arg(short, long)]
        permanent: bool,
        /// 设为手动启动 (Manual)
        #[arg(short, long)]
        manual: bool,
    },
    /// 关闭服务
    Stop {
        services: Vec<String>,
        /// 设为禁用 (Disabled)
        #[arg(short, long)]
        permanent: bool,
        /// 设为手动 (Manual)
        #[arg(short, long)]
        manual: bool,
    },
    /// 列出当前运行的进程，支持关键字过滤 (例如: wsm ps chrome)
    Ps { filter: Option<String> },
    /// 终止进程，支持进程名或 PID，支持逗号分隔 (例如: wsm kill notepad,1234)
    Kill { processes: Vec<String> },
    /// 以 MCP Server 模式运行（stdio JSON-RPC）
    Mcp,
}

fn main() {
    let cli = Cli::parse();

    let manager = match ServiceManager::new() {
        Ok(m) => m,
        Err(e) => {
            eprintln!("初始化失败: {}", e);
            return;
        }
    };
    let proc_manager = ProcessManager::new();

    match cli.command {
        Commands::List { filter } => {
            if let Err(e) = manager.list_services(filter, true) {
                eprintln!("查询失败: {}", e);
            }
        }
        Commands::ListAll { filter } => {
            if let Err(e) = manager.list_services(filter, false) {
                eprintln!("查询失败: {}", e);
            }
        }
        Commands::Open {
            services,
            permanent,
            manual,
        } => {
            let expanded_services = expand_services(services);
            process_batch(&manager, expanded_services, true, permanent, manual);
        }
        Commands::Stop {
            services,
            permanent,
            manual,
        } => {
            let expanded_services = expand_services(services);
            process_batch(&manager, expanded_services, false, permanent, manual);
        }
        Commands::Ps { filter } => match proc_manager.list_processes(filter.as_deref()) {
            Ok(list) => print!("{}", proc_manager.format_list(&list)),
            Err(e) => eprintln!("查询失败: {}", e),
        },
        Commands::Kill { processes } => {
            let targets: Vec<String> = processes
                .iter()
                .flat_map(|s| s.split(','))
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            if targets.is_empty() {
                println!("⚠️ 未输入任何进程名称或 PID");
                return;
            }
            println!("🚀 开始批量终止 {} 个目标...", targets.len());
            println!("{}", "-".repeat(60));
            for (target, result) in proc_manager.kill_processes(targets) {
                match result {
                    Ok(_) => {
                        println!("✅ 成功终止: {}", target);
                        logger::log_action(&target, "PS_KILL", "SUCCESS");
                    }
                    Err(e) if e.contains("[PROTECTED]") => {
                        println!("[ 🛡️  忽略 ] {} — {}", target, e);
                        logger::log_action(&target, "PS_KILL", "IGNORED (PROTECTED)");
                    }
                    Err(e) => {
                        println!("❌ 失败: {} — {}", target, e);
                        logger::log_action(&target, "PS_KILL", &format!("FAILED: {}", e));
                    }
                }
            }
        }
        Commands::Mcp => {
            eprintln!("[INFO] win-service-control-mcp 启动 (MCP 模式)");
            mcp_server::run_loop(manager, proc_manager);
        }
    }
}

fn expand_services(input: Vec<String>) -> Vec<String> {
    input
        .iter()
        .flat_map(|s| s.split(','))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

fn process_batch(
    manager: &ServiceManager,
    services: Vec<String>,
    start: bool,
    permanent: bool,
    manual: bool,
) {
    if services.is_empty() {
        println!("⚠️ 未输入任何服务名称");
        return;
    }

    println!("🚀 开始批量处理 {} 个服务...", services.len());
    println!("{}", "-".repeat(60));

    let action_label = if start { "OPEN" } else { "STOP" };
    let mode_suffix = if manual {
        " (MANUAL)"
    } else if permanent {
        if start {
            " (AUTO)"
        } else {
            " (DISABLED)"
        }
    } else {
        ""
    };

    for (i, svc) in services.iter().enumerate() {
        match manager.set_state(svc, start, permanent, manual) {
            Ok(full_name) => {
                println!(
                    "[{}/{}] ✅ 成功: {}{}",
                    i + 1,
                    services.len(),
                    full_name,
                    mode_suffix
                );
                logger::log_action(
                    &full_name,
                    &format!("{}{}", action_label, mode_suffix),
                    "SUCCESS",
                );
            }
            Err(e) if e.contains("[PROTECTED]") => {
                println!("[ 🛡️  忽略 ] {}", e);
                logger::log_action(svc, action_label, "IGNORED (SYSTEM PROTECTED)");
            }
            Err(e) => {
                println!("[{}/{}] ❌ 失败: {}: {}", i + 1, services.len(), svc, e);
                logger::log_action(svc, action_label, &format!("FAILED: {}", e));
            }
        }
    }
}
