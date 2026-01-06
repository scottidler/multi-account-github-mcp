//! Configuration handling for multi-account-github-mcp

use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Account configuration with token file path
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AccountConfig {
    /// Path to the token file (supports ~ expansion)
    pub token_path: String,
}

/// Main configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    /// Default account to use when none specified
    pub default_account: String,

    /// Map of account names to their configurations
    pub accounts: HashMap<String, AccountConfig>,
}

impl Default for Config {
    fn default() -> Self {
        let mut accounts = HashMap::new();
        accounts.insert(
            "home".to_string(),
            AccountConfig {
                token_path: "~/.config/github/tokens/scottidler".to_string(),
            },
        );
        Self {
            default_account: "home".to_string(),
            accounts,
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

    /// Get an account configuration by name, or the default if None
    pub fn get_account(&self, name: Option<&str>) -> Result<&AccountConfig> {
        let account_name = name.unwrap_or(&self.default_account);
        self.accounts
            .get(account_name)
            .ok_or_else(|| Error::AccountNotFound(account_name.to_string()))
    }

    /// Get the token for an account by reading the token file
    pub fn get_token(&self, account: Option<&str>) -> Result<String> {
        let account_config = self.get_account(account)?;
        let expanded_path = shellexpand::tilde(&account_config.token_path);
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
        assert_eq!(config.default_account, "home");
        assert!(config.accounts.contains_key("home"));
    }

    #[test]
    fn test_load_from_yaml() {
        let yaml = r#"
default_account: work
accounts:
  home:
    token_path: ~/.config/github/tokens/personal
  work:
    token_path: ~/.config/github/tokens/work
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
    fn test_get_account_default() {
        let config = Config::default();
        let account = config.get_account(None).unwrap();
        assert!(account.token_path.contains("scottidler"));
    }

    #[test]
    fn test_get_account_not_found() {
        let config = Config::default();
        let result = config.get_account(Some("nonexistent"));
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
  test:
    token_path: {}
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
  test:
    token_path: {}
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
