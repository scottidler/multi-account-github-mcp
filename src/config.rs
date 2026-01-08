//! Configuration handling for multi-account-github-mcp

use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Logging configuration
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct LogConfig {
    /// Log level: trace, debug, info, warn, error (default: info)
    #[serde(default = "default_log_level")]
    pub level: String,

    /// Optional log file path (supports ~ expansion). If not set, logs to stderr.
    #[serde(default)]
    pub file: Option<String>,
}

fn default_log_level() -> String {
    "info".to_string()
}

/// Main configuration
/// Simple format: accounts map directly to token file paths
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    /// Default account to use when none specified
    #[serde(default = "default_account")]
    pub default_account: String,

    /// Map of account names to token file paths (supports ~ expansion)
    pub accounts: HashMap<String, String>,

    /// Logging configuration
    #[serde(default)]
    pub logging: LogConfig,
}

fn default_account() -> String {
    "default".to_string()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            default_account: "default".to_string(),
            accounts: HashMap::new(),
            logging: LogConfig::default(),
        }
    }
}

impl Config {
    /// Load configuration with fallback chain:
    /// 1. Explicit path from CLI
    /// 2. ~/.config/multi-account-github-mcp/multi-account-github-mcp.yml
    /// 3. ./multi-account-github-mcp.yml
    pub fn load(config_path: Option<&PathBuf>) -> Result<Self> {
        // If explicit config path provided, try to load it
        if let Some(path) = config_path {
            return Self::load_from_file(path);
        }

        // Try primary location: ~/.config/<project>/<project>.yml
        if let Some(config_dir) = dirs::config_dir() {
            let project_name = env!("CARGO_PKG_NAME");
            let primary_config = config_dir.join(project_name).join(format!("{project_name}.yml"));
            if primary_config.exists() {
                match Self::load_from_file(&primary_config) {
                    Ok(config) => return Ok(config),
                    Err(e) => {
                        tracing::warn!("Failed to load config from {}: {}", primary_config.display(), e);
                    }
                }
            }
        }

        // Try fallback location: ./<project>.yml
        let project_name = env!("CARGO_PKG_NAME");
        let fallback_config = PathBuf::from(format!("{project_name}.yml"));
        if fallback_config.exists() {
            match Self::load_from_file(&fallback_config) {
                Ok(config) => return Ok(config),
                Err(e) => {
                    tracing::warn!("Failed to load config from {}: {}", fallback_config.display(), e);
                }
            }
        }

        // No config file found, use defaults
        tracing::info!("No config file found, using defaults");
        Ok(Self::default())
    }

    fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(&path)
            .map_err(|e| Error::Config(format!("Failed to read config file {}: {}", path.as_ref().display(), e)))?;

        let config: Self = serde_yaml::from_str(&content)?;

        tracing::info!("Loaded config from: {}", path.as_ref().display());
        Ok(config)
    }

    /// Get the token path for an account by name, or the default if None
    pub fn get_token_path(&self, name: Option<&str>) -> Result<&str> {
        let account_name = name.unwrap_or(&self.default_account);
        self.accounts
            .get(account_name)
            .map(|s| s.as_str())
            .ok_or_else(|| Error::AccountNotFound(account_name.to_string()))
    }

    /// Get the token for an account by reading the token file
    pub fn get_token(&self, account: Option<&str>) -> Result<String> {
        let token_path = self.get_token_path(account)?;
        let expanded_path = shellexpand::tilde(token_path);
        let path = PathBuf::from(expanded_path.as_ref());

        if !path.exists() {
            return Err(Error::TokenNotFound(path.display().to_string()));
        }

        let token = fs::read_to_string(&path)
            .map_err(|e| Error::TokenRead(format!("{}: {}", path.display(), e)))?
            .trim()
            .to_string();

        if token.is_empty() {
            return Err(Error::TokenRead(format!("Token file is empty: {}", path.display())));
        }

        Ok(token)
    }

    /// List all configured account names
    pub fn account_names(&self) -> Vec<&str> {
        self.accounts.keys().map(|s| s.as_str()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.default_account, "default");
        assert!(config.accounts.is_empty());
    }

    #[test]
    fn test_load_from_yaml() {
        let yaml = r#"
default_account: work
accounts:
  home: ~/.config/github/tokens/personal
  work: ~/.config/github/tokens/work
"#;

        let mut file = NamedTempFile::new().unwrap();
        file.write_all(yaml.as_bytes()).unwrap();

        let config = Config::load_from_file(file.path()).unwrap();
        assert_eq!(config.default_account, "work");
        assert_eq!(config.accounts.len(), 2);
        assert!(config.accounts.contains_key("home"));
        assert!(config.accounts.contains_key("work"));
    }

    #[test]
    fn test_get_token_path_default() {
        let mut accounts = HashMap::new();
        accounts.insert("home".to_string(), "/path/to/token".to_string());
        let config = Config {
            default_account: "home".to_string(),
            accounts,
            logging: LogConfig::default(),
        };
        let path = config.get_token_path(None).unwrap();
        assert_eq!(path, "/path/to/token");
    }

    #[test]
    fn test_get_account_not_found() {
        let config = Config::default();
        let result = config.get_token_path(Some("nonexistent"));
        assert!(matches!(result, Err(Error::AccountNotFound(_))));
    }

    #[test]
    fn test_get_token_from_file() {
        let mut token_file = NamedTempFile::new().unwrap();
        token_file.write_all(b"ghp_test_token_12345").unwrap();

        let yaml = format!(
            r#"
default_account: test
accounts:
  test: {}
"#,
            token_file.path().display()
        );

        let mut config_file = NamedTempFile::new().unwrap();
        config_file.write_all(yaml.as_bytes()).unwrap();

        let config = Config::load_from_file(config_file.path()).unwrap();
        let token = config.get_token(Some("test")).unwrap();
        assert_eq!(token, "ghp_test_token_12345");
    }

    #[test]
    fn test_token_trimmed() {
        let mut token_file = NamedTempFile::new().unwrap();
        token_file.write_all(b"  ghp_token_with_whitespace  \n").unwrap();

        let yaml = format!(
            r#"
default_account: test
accounts:
  test: {}
"#,
            token_file.path().display()
        );

        let mut config_file = NamedTempFile::new().unwrap();
        config_file.write_all(yaml.as_bytes()).unwrap();

        let config = Config::load_from_file(config_file.path()).unwrap();
        let token = config.get_token(Some("test")).unwrap();
        assert_eq!(token, "ghp_token_with_whitespace");
    }
}
