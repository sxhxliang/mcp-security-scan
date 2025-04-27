// mod models;
mod mcp_client;
mod mcp_types;
mod storage_file;
mod utils;
mod verify_api;
mod scan;
mod cli;

use anyhow::Ok;
use clap::Parser;

use scan::MCPScanner;
use colored::*;


use cli::{Cli, Commands, ScanArgs, CommonArgs};

// const VERSION: &str = env!("CARGO_PKG_VERSION");
// const DEFAULT_STORAGE_PATH: &str = "~/.mcp-scan";
// 平台相关路径
fn well_known_mcp_paths() -> Vec<String> {
    let mut paths = vec![
        "~/.codeium/windsurf/mcp_config.json".to_string(),
        "~/.cursor/mcp.json".to_string(),
    ];

    if cfg!(target_os = "linux") {
        paths.extend(vec![
            "~/.vscode/mcp.json".to_string(),
            "~/.config/Code/User/settings.json".to_string(),
        ]);
    } else if cfg!(target_os = "macos") {
        paths.extend(vec![
            "~/Library/Application Support/Claude/claude_desktop_config.json".to_string(),
            "~/.vscode/mcp.json".to_string(),
            "~/Library/Application Support/Code/User/settings.json".to_string(),
        ]);
    } else if cfg!(target_os = "windows") {
        paths.extend(vec![
            "~/AppData/Roaming/Claude/claude_desktop_config.json".to_string(),
            "~/.vscode/mcp.json".to_string(),
            "~/AppData/Roaming/Code/User/settings.json".to_string(),
        ]);
    }
    paths
}
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    
    // 显示版本信息
    println!("{}", format!("AgentX MCP-scan v{}", env!("CARGO_PKG_VERSION"))
        .bright_blue().bold());

    match cli.command.unwrap_or(Commands::Scan(ScanArgs { 
        common: CommonArgs { 
            storage_file: "~/.mcp-scan".into(), 
            base_url: "".into()
        },
        server_timeout: 10,
        suppress_mcpserver_io: true,
        checks_per_server: 1,
        files: Vec::new()
    })) {
        Commands::Scan(args) => {
            let files = if args.files.is_empty() {
                well_known_mcp_paths()
            } else {
                args.files
            };
            
            let scanner = MCPScanner::new(
                &args.common.storage_file,
                &args.common.base_url,
                args.checks_per_server,
                args.suppress_mcpserver_io,
                args.server_timeout as usize,
            );
            scanner.scan_files(&files).await;
        }
        Commands::Inspect(args) => {
            let files = if args.files.is_empty() {
                well_known_mcp_paths()
            } else {
                args.files
            };
            
            let scanner = MCPScanner::new(
                &args.common.storage_file,
                &args.common.base_url,
                args.server_timeout,
                false,
                args.server_timeout as usize,
            );
            scanner.inspect().await?;
        }
        Commands::Whitelist(args) => {
            let scanner = MCPScanner::new(
                &args.common.storage_file,
                &args.common.base_url,
                1, // checks_per_server not used
                false,
                10 as usize,
            );

            // if args.reset {
            //     scanner.reset_whitelist().await;
            //     process::exit(0);
            // }

            // match (args.name, args.hash) {
            //     (Some(name), Some(hash)) => {
            //         scanner.whitelist(&name, &hash, args.local_only).await;
            //         scanner.print_whitelist().await;
            //     }
            //     (None, None) => {
            //         scanner.print_whitelist().await;
            //     }
            //     _ => {
            //         eprintln!("\x1b[31;1mPlease provide both name and hash\x1b[0m");
            //         process::exit(1);
            //     }
            // }
        }
    }
    Ok(())
}