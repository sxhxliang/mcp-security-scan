
use clap::{Args, Parser, Subcommand};


#[derive(Parser)]
#[command(name = "mcp-scan")]
#[command(version)]
#[command(about = "MCP-scan: Security scanner for Model Context Protocol servers and tools")]
#[command(long_about = "MCP-scan: Security scanner for Model Context Protocol servers and tools

Examples:
  mcp-scan                     # Scan all known MCP configs
  mcp-scan ~/custom/config.json # Scan a specific config file
  mcp-scan inspect             # Just inspect tools without verification
  mcp-scan whitelist           # View whitelisted tools
  mcp-scan whitelist tool \"add\" \"a1b2c3...\" # Whitelist the 'add' tool")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Scan MCP servers for security issues [default]
    Scan(ScanArgs),
    /// Print descriptions without verification
    Inspect(InspectArgs),
    /// Manage the whitelist of approved entities
    Whitelist(WhitelistArgs),
}

#[derive(Args)]
pub struct CommonArgs {
    /// Path to store scan results and whitelist
    #[arg(long, default_value = "~/.mcp-security-scan")]
    pub storage_file: String,
    
    /// Base URL for verification server
    #[arg(long, default_value = "")]
    pub base_url: String,
}

#[derive(Args)]
pub struct ScanArgs {
    #[command(flatten)]
    pub common: CommonArgs,
    
    /// Seconds to wait for server connections
    #[arg(long, default_value = "10")]
    pub server_timeout: i64,
    
    /// Suppress MCP server output
    #[arg(long, default_value = "true")]
    pub suppress_mcpserver_io: bool,
    
    /// Number of checks per server
    #[arg(long, default_value = "1")]
    pub checks_per_server: i64,
    pub files: Vec<String>,
}

#[derive(Args)]
pub struct InspectArgs {
    #[command(flatten)]
    pub common: CommonArgs,
    
    /// Seconds to wait for server connections
    #[arg(long, default_value = "10")]
    pub server_timeout: i64,
    pub languages: Option<String>,
    pub files: Vec<String>,
}

#[derive(Args)]
pub struct WhitelistArgs {
    #[command(flatten)]
    pub common: CommonArgs,
    
    /// Reset the entire whitelist
    #[arg(long)]
    pub reset: bool,
    
    /// Only update local whitelist
    #[arg(long)]
    local_only: bool,
    
    /// Type of entity to whitelist
    entity_type: Option<String>,
    
    /// Name of the entity
    entity_name: Option<String>,
    
    /// Hash of the entity
    entity_hash: Option<String>,
}