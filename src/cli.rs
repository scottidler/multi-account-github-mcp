//! CLI argument parsing for multi-account-github-mcp

use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// Check if required tools are available and format their versions
fn check_required_tools() -> String {
    let gh_status = match std::process::Command::new("gh").arg("--version").output() {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout);
            let version = version.lines().next().unwrap_or("unknown");
            // Extract just the version number (e.g., "2.45.0" from "gh version 2.45.0 (2024-01-01)")
            let version = version
                .strip_prefix("gh version ")
                .unwrap_or(version)
                .split_whitespace()
                .next()
                .unwrap_or("unknown");
            format!("  ✅ gh       {version}")
        }
        _ => "  ❌ gh       NOT FOUND - Install from https://cli.github.com".to_string(),
    };

    format!("REQUIRED TOOLS:\n{gh_status}")
}

#[derive(Parser)]
#[command(
    name = "multi-account-github-mcp",
    about = "GitHub MCP server with multi-account support",
    version = env!("GIT_DESCRIBE"),
    after_help = check_required_tools(),
)]
pub struct Cli {
    /// Path to config file
    #[arg(short, long, global = true, help = "Path to config file")]
    pub config: Option<PathBuf>,

    /// Enable verbose output
    #[arg(short, long, global = true, help = "Enable verbose output")]
    pub verbose: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Start the MCP server (stdio transport)
    Serve,

    /// List configured accounts
    Accounts,

    /// Test connection for an account
    Test {
        /// Account name to test (uses default if not specified)
        #[arg(help = "Account name to test")]
        account: Option<String>,
    },
}
