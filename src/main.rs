//! multi-account-github-mcp - GitHub MCP server with multi-account support

use clap::Parser;
use eyre::{Context, Result};
use multi_account_github_mcp::{Config, GhClient};
use rmcp::ServiceExt;
use std::io::{self, Write};
use tracing_subscriber::EnvFilter;

mod cli;

use cli::{Cli, Commands};

fn setup_logging(verbose: bool) -> Result<()> {
    let filter = if verbose { EnvFilter::new("debug") } else { EnvFilter::new("info") };

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(io::stderr)
        .init();

    Ok(())
}

async fn run_serve(config: Config) -> Result<()> {
    let gh = GhClient::new(config).context("Failed to create GitHub client")?;
    let server = multi_account_github_mcp::mcp::GitHubMcpServer::new(gh);

    tracing::info!("Starting MCP server on stdio");

    let transport = (tokio::io::stdin(), tokio::io::stdout());
    let service = server.serve(transport).await?;
    service.waiting().await?;

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

    // Setup logging (only for non-serve commands, serve uses stderr)
    if !matches!(cli.command, Commands::Serve) {
        setup_logging(cli.verbose)?;
    }

    // Load configuration
    let config = Config::load(cli.config.as_ref()).context("Failed to load configuration")?;

    match cli.command {
        Commands::Serve => {
            // For serve, setup minimal logging to stderr
            setup_logging(cli.verbose)?;
            run_serve(config).await
        }
        Commands::Accounts => run_accounts(&config),
        Commands::Test { account } => run_test(config, account).await,
    }
}
