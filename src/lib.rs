//! multi-account-github-mcp library
//!
//! A GitHub MCP server with multi-account support, wrapping the gh CLI.

pub mod config;
pub mod error;
pub mod gh;
pub mod mcp;
pub mod tools;

pub use config::{Config, LogConfig};
pub use error::Error;
pub use gh::GhClient;

pub type Result<T> = std::result::Result<T, Error>;
