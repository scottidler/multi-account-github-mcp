//! multi-account-github-mcp - GitHub MCP server with multi-account support

use clap::Parser;
use eyre::{Context, Result};
use multi_account_github_mcp::{Config, LogConfig, GhClient};
use rmcp::ServiceExt;
use std::io::{self, Write};
use std::path::PathBuf;
use tracing_subscriber::EnvFilter;

mod cli;

use cli::{Cli, Commands};

fn setup_logging(verbose: bool, log_config: &LogConfig) -> Result<()> {
    // Determine log level: CLI verbose flag overrides config
    let level = if verbose {
        "debug"
    } else {
        &log_config.level
    };
    let filter = EnvFilter::new(level);

    // If log file is configured, write to file; otherwise write to stderr
    if let Some(ref log_file) = log_config.file {
        let expanded_path = shellexpand::tilde(log_file);
        let path = PathBuf::from(expanded_path.as_ref());

        // Create parent directories if needed
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)?;

        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_writer(file)
            .with_ansi(false)
            .init();

        eprintln!("Logging to: {}", path.display());
    } else {
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_writer(io::stderr)
            .init();
    }

    Ok(())
}

async fn run_serve(config: Config) -> Result<()> {
    tracing::debug!("Creating GitHub client with config: {:?}", config);
    let gh = GhClient::new(config).context("Failed to create GitHub client")?;
    tracing::debug!("GitHub client created successfully");

    tracing::info!("Creating MCP server");
    let server = multi_account_github_mcp::mcp::GitHubMcpServer::new(gh);
    tracing::debug!("MCP server created");

    tracing::info!("Starting MCP server on stdio transport");
    let transport = (tokio::io::stdin(), tokio::io::stdout());

    tracing::debug!("Calling server.serve() to start MCP protocol");
    let service = server.serve(transport).await?;
    tracing::info!("MCP server started, waiting for requests...");

    service.waiting().await?;
    tracing::info!("MCP server shutting down");

    Ok(())
}

fn run_accounts(config: &Config) -> Result<()> {
    println!("Configured accounts:");
    println!();

    let mut accounts: Vec<_> = config.accounts.iter().collect();
    accounts.sort_by_key(|(name, _)| *name);

    for (name, token_path) in accounts {
        let is_default = name == &config.default_account;
        let default_marker = if is_default { " (default)" } else { "" };

        // Check if token file exists
        let expanded_path = shellexpand::tilde(token_path);
        let path = std::path::PathBuf::from(expanded_path.as_ref());
        let status = if path.exists() { "✅" } else { "❌" };

        println!("  {status} {name}{default_marker}");
        println!("     Token: {token_path}");
    }

    Ok(())
}

async fn run_test(config: Config, account: Option<String>) -> Result<()> {
    let account_name = account.as_deref().unwrap_or(&config.default_account);
    println!("Testing account: {account_name}");
    println!();

    let gh = GhClient::new(config).context("Failed to create GitHub client")?;

    // Test gh version
    print!("Checking gh CLI... ");
    io::stdout().flush()?;
    match gh.version().await {
        Ok(version) => println!("✅ {version}"),
        Err(e) => {
            println!("❌ {e}");
            return Ok(());
        }
    }

    // Test authentication
    print!("Testing authentication... ");
    io::stdout().flush()?;
    match gh.run(account.as_deref(), &["api", "user", "--jq", ".login"]).await {
        Ok(login) => {
            let login = login.as_str().unwrap_or("unknown").trim_matches('"');
            println!("✅ Authenticated as: {login}");
        }
        Err(e) => {
            println!("❌ {e}");
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Load configuration first (needed for logging setup)
    let config = Config::load(cli.config.as_ref()).context("Failed to load configuration")?;

    // Setup logging with config
    setup_logging(cli.verbose, &config.logging)?;

    match cli.command {
        Commands::Serve => run_serve(config).await,
        Commands::Accounts => run_accounts(&config),
        Commands::Test { account } => run_test(config, account).await,
    }
}
