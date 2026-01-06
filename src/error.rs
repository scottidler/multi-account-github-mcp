//! Error types for multi-account-github-mcp

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Account not found: {0}")]
    AccountNotFound(String),

    #[error("Token file not found: {0}")]
    TokenNotFound(String),

    #[error("Token file read error: {0}")]
    TokenRead(String),

    #[error("gh CLI error: {0}")]
    GhCli(String),

    #[error("gh CLI not found. Install from https://cli.github.com")]
    GhNotFound,

    #[error("JSON parse error: {0}")]
    JsonParse(#[from] serde_json::Error),

    #[error("YAML parse error: {0}")]
    YamlParse(#[from] serde_yaml::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("MCP error: {0}")]
    Mcp(String),

    #[error("Tool error: {0}")]
    Tool(String),
}
