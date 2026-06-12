//! Configuration handling for multi-account-github-mcp

use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// XDG config dir, honoring `$XDG_CONFIG_HOME` and falling back to `$HOME/.config`.
///
/// We deliberately do NOT use the `dirs` config/data helpers: those honor
/// `$XDG_CONFIG_HOME` / `$XDG_DATA_HOME` only on Linux. On macOS they resolve via system
/// APIs and return `~/Library/...`, ignoring the env vars. These helpers resolve to the
/// same XDG layout on every platform.
fn xdg_config_dir() -> Option<PathBuf> {
    if let Ok(dir) = std::env::var("XDG_CONFIG_HOME") {
        let path = PathBuf::from(dir);
        if path.is_absolute() {
            return Some(path);
        }
    }
    dirs::home_dir().map(|h| h.join(".config"))
}

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

/// Rate limiting configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct RateLimitConfig {
    #[serde(default = "default_requests_per_minute")]
    pub requests_per_minute: u32,

    #[serde(default = "default_search_requests_per_minute")]
    pub search_requests_per_minute: u32,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_minute: default_requests_per_minute(),
            search_requests_per_minute: default_search_requests_per_minute(),
        }
    }
}

fn default_requests_per_minute() -> u32 {
    80
}

fn default_search_requests_per_minute() -> u32 {
    25
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

    /// Rate limiting configuration
    #[serde(default, rename = "rate-limit")]
    pub rate_limit: RateLimitConfig,
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
            rate_limit: RateLimitConfig::default(),
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
        if let Some(config_dir) = xdg_config_dir() {
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

    /// Get the token source for an account by name, or the default if None.
    /// The source may be an `env:VAR_NAME` reference or a file path.
    pub fn get_token_source(&self, name: Option<&str>) -> Result<&str> {
        let account_name = name.unwrap_or(&self.default_account);
        self.accounts
            .get(account_name)
            .map(|s| s.as_str())
            .ok_or_else(|| Error::AccountNotFound(account_name.to_string()))
    }

    /// Get the token for an account.
    /// If the source starts with `env:`, read from the named environment variable.
    /// Otherwise, read from a file path (existing behavior).
    pub fn get_token(&self, account: Option<&str>) -> Result<String> {
        let source = self.get_token_source(account)?;

        if let Some(var_name) = source.strip_prefix("env:") {
            let token = std::env::var(var_name)
                .map_err(|_| Error::EnvVarNotFound(var_name.to_string()))?
                .trim()
                .to_string();
            if token.is_empty() {
                return Err(Error::EnvVarNotFound(var_name.to_string()));
            }
            return Ok(token);
        }

        let expanded_path = shellexpand::tilde(source);
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
    fn test_get_token_source_default() {
        let mut accounts = HashMap::new();
        accounts.insert("home".to_string(), "/path/to/token".to_string());
        let config = Config {
            default_account: "home".to_string(),
            accounts,
            logging: LogConfig::default(),
            rate_limit: RateLimitConfig::default(),
        };
        let source = config.get_token_source(None).unwrap();
        assert_eq!(source, "/path/to/token");
    }

    #[test]
    fn test_get_account_not_found() {
        let config = Config::default();
        let result = config.get_token_source(Some("nonexistent"));
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

    #[test]
    fn test_get_token_from_env_var() {
        let mut accounts = HashMap::new();
        accounts.insert("test".to_string(), "env:TEST_GH_TOKEN_ABC".to_string());
        let config = Config {
            default_account: "test".to_string(),
            accounts,
            logging: LogConfig::default(),
            rate_limit: RateLimitConfig::default(),
        };

        temp_env::with_var("TEST_GH_TOKEN_ABC", Some("ghp_env_token_12345"), || {
            let token = config.get_token(Some("test")).unwrap();
            assert_eq!(token, "ghp_env_token_12345");
        });
    }

    #[test]
    fn test_get_token_env_var_missing() {
        let mut accounts = HashMap::new();
        accounts.insert("test".to_string(), "env:TEST_GH_MISSING_VAR".to_string());
        let config = Config {
            default_account: "test".to_string(),
            accounts,
            logging: LogConfig::default(),
            rate_limit: RateLimitConfig::default(),
        };

        temp_env::with_var_unset("TEST_GH_MISSING_VAR", || {
            let result = config.get_token(Some("test"));
            assert!(matches!(result, Err(Error::EnvVarNotFound(_))));
        });
    }

    #[test]
    fn test_get_token_env_var_empty() {
        let mut accounts = HashMap::new();
        accounts.insert("test".to_string(), "env:TEST_GH_EMPTY_VAR".to_string());
        let config = Config {
            default_account: "test".to_string(),
            accounts,
            logging: LogConfig::default(),
            rate_limit: RateLimitConfig::default(),
        };

        temp_env::with_var("TEST_GH_EMPTY_VAR", Some(""), || {
            let result = config.get_token(Some("test"));
            assert!(matches!(result, Err(Error::EnvVarNotFound(_))));
        });
    }

    #[test]
    fn test_get_token_env_var_trimmed() {
        let mut accounts = HashMap::new();
        accounts.insert("test".to_string(), "env:TEST_GH_TRIM_VAR".to_string());
        let config = Config {
            default_account: "test".to_string(),
            accounts,
            logging: LogConfig::default(),
            rate_limit: RateLimitConfig::default(),
        };

        temp_env::with_var("TEST_GH_TRIM_VAR", Some("  ghp_trimmed  \n"), || {
            let token = config.get_token(Some("test")).unwrap();
            assert_eq!(token, "ghp_trimmed");
        });
    }

    #[test]
    fn test_file_path_unchanged_with_env_prefix_feature() {
        // Ensure regular file paths still work alongside env: support
        let mut token_file = NamedTempFile::new().unwrap();
        token_file.write_all(b"ghp_file_token").unwrap();

        let mut accounts = HashMap::new();
        accounts.insert("file_account".to_string(), token_file.path().display().to_string());
        let config = Config {
            default_account: "file_account".to_string(),
            accounts,
            logging: LogConfig::default(),
            rate_limit: RateLimitConfig::default(),
        };

        let token = config.get_token(Some("file_account")).unwrap();
        assert_eq!(token, "ghp_file_token");
    }

    #[test]
    fn test_rate_limit_defaults() {
        let config = Config::default();
        assert_eq!(config.rate_limit.requests_per_minute, 80);
        assert_eq!(config.rate_limit.search_requests_per_minute, 25);
    }

    #[test]
    fn test_rate_limit_from_yaml() {
        let yaml = r#"
default-account: home
accounts:
  home: /tmp/token
rate-limit:
  requests-per-minute: 60
  search-requests-per-minute: 10
"#;

        let mut file = NamedTempFile::new().unwrap();
        file.write_all(yaml.as_bytes()).unwrap();

        let config = Config::load_from_file(file.path()).unwrap();
        assert_eq!(config.rate_limit.requests_per_minute, 60);
        assert_eq!(config.rate_limit.search_requests_per_minute, 10);
    }

    #[test]
    fn test_rate_limit_missing_section_uses_defaults() {
        let yaml = r#"
default-account: home
accounts:
  home: /tmp/token
"#;

        let mut file = NamedTempFile::new().unwrap();
        file.write_all(yaml.as_bytes()).unwrap();

        let config = Config::load_from_file(file.path()).unwrap();
        assert_eq!(config.rate_limit.requests_per_minute, 80);
        assert_eq!(config.rate_limit.search_requests_per_minute, 25);
    }

    #[test]
    fn test_rate_limit_partial_uses_defaults_for_missing() {
        let yaml = r#"
default-account: home
accounts:
  home: /tmp/token
rate-limit:
  requests-per-minute: 50
"#;

        let mut file = NamedTempFile::new().unwrap();
        file.write_all(yaml.as_bytes()).unwrap();

        let config = Config::load_from_file(file.path()).unwrap();
        assert_eq!(config.rate_limit.requests_per_minute, 50);
        assert_eq!(config.rate_limit.search_requests_per_minute, 25);
    }
}
