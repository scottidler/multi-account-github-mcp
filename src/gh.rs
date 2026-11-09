//! gh CLI wrapper for multi-account-github-mcp

use crate::{Config, Error, Result};
use dashmap::DashMap;
use governor::clock::DefaultClock;
use governor::state::{InMemoryState, NotKeyed};
use governor::{Quota, RateLimiter as GovRateLimiter};
use serde_json::Value;
use std::num::NonZeroU32;
use std::process::Stdio;
use std::sync::Arc;
use tokio::process::Command;

type Limiter = GovRateLimiter<NotKeyed, InMemoryState, DefaultClock>;

/// Extract rate limit reset timestamp from GitHub error message
fn extract_rate_limit_reset(error_msg: &str) -> String {
    if let Some(idx) = error_msg.find("timestamp ") {
        let start = idx + "timestamp ".len();
        if let Some(end) = error_msg[start..].find(" UTC") {
            return format!("{} UTC", &error_msg[start..start + end]);
        }
    }
    "unknown".to_string()
}

/// Classify a gh CLI error into a specific error variant
fn classify_gh_error(error_msg: &str, account: Option<&str>, args: &[&str]) -> Error {
    let account_label = account.unwrap_or("default");
    let command = args.join(" ");

    // Check rate limit first (most specific)
    if error_msg.contains("rate limit exceeded")
        || error_msg.contains("secondary rate limit")
        || (error_msg.contains("HTTP 403") && error_msg.contains("rate limit"))
    {
        let reset_at = extract_rate_limit_reset(error_msg);
        tracing::warn!(
            account = account_label,
            command = %command,
            reset = %reset_at,
            "GitHub API rate limit exceeded"
        );
        return Error::RateLimit {
            account: account_label.to_string(),
            reset_at,
        };
    }

    // Check scope errors
    if error_msg.contains("missing_scope") || error_msg.contains("insufficient_scope") {
        tracing::warn!(
            account = account_label,
            command = %command,
            "GitHub API scope error - PAT may lack required permissions"
        );
        return Error::GhCli(format!(
            "Missing OAuth scope for account '{}'. Command: gh {}. \
             Check that the PAT has the required scopes. Error: {}",
            account_label,
            command,
            error_msg.trim()
        ));
    }

    // Generic error (existing behavior)
    Error::GhCli(error_msg.trim().to_string())
}

fn make_limiter(requests_per_minute: u32) -> Arc<Limiter> {
    let rpm = requests_per_minute.max(1);
    let quota = Quota::per_minute(NonZeroU32::new(rpm).expect("rpm is at least 1"));
    Arc::new(GovRateLimiter::direct(quota))
}

/// Client for executing gh CLI commands with account-specific tokens
#[derive(Debug, Clone)]
pub struct GhClient {
    config: Arc<Config>,
    general_limiters: Arc<DashMap<String, Arc<Limiter>>>,
    search_limiters: Arc<DashMap<String, Arc<Limiter>>>,
}

impl GhClient {
    /// Create a new GhClient with the given configuration
    pub fn new(config: Config) -> Result<Self> {
        // Verify gh is installed
        if which::which("gh").is_err() {
            return Err(Error::GhNotFound);
        }

        Ok(Self {
            config: Arc::new(config),
            general_limiters: Arc::new(DashMap::new()),
            search_limiters: Arc::new(DashMap::new()),
        })
    }

    /// Get the underlying config
    pub fn config(&self) -> &Config {
        &self.config
    }

    fn get_limiter(&self, account: Option<&str>, is_search: bool) -> Arc<Limiter> {
        let key = account.unwrap_or(&self.config.default_account).to_string();
        let map = if is_search { &self.search_limiters } else { &self.general_limiters };
        let rpm = if is_search {
            self.config.rate_limit.search_requests_per_minute
        } else {
            self.config.rate_limit.requests_per_minute
        };
        map.entry(key).or_insert_with(|| make_limiter(rpm)).clone()
    }

    /// Run a gh command with the specified account's token
    ///
    /// # Arguments
    /// * `account` - Optional account name; uses default if None
    /// * `args` - Command arguments to pass to gh
    ///
    /// # Returns
    /// Parsed JSON output from gh command
    pub async fn run(&self, account: Option<&str>, args: &[&str]) -> Result<Value> {
        let is_search = args.first().is_some_and(|a| *a == "search");

        // All requests go through the general limiter
        let general = self.get_limiter(account, false);
        if general.check().is_err() {
            tracing::debug!(
                account = account.unwrap_or("default"),
                command = %args.join(" "),
                "Rate limited - waiting for general quota"
            );
        }
        general.until_ready().await;

        // Search requests additionally go through the search limiter
        if is_search {
            let search = self.get_limiter(account, true);
            if search.check().is_err() {
                tracing::debug!(
                    account = account.unwrap_or("default"),
                    command = %args.join(" "),
                    "Rate limited - waiting for search quota"
                );
            }
            search.until_ready().await;
        }

        let token = self.config.get_token(account)?;

        tracing::debug!(
            "Running gh command with account {:?}: gh {}",
            account.unwrap_or("default"),
            args.join(" ")
        );

        let output = Command::new("gh")
            .args(args)
            .env("GH_TOKEN", &token)
            .env("NO_COLOR", "1")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| Error::GhCli(format!("Failed to spawn gh: {e}")))?
            .wait_with_output()
            .await
            .map_err(|e| Error::GhCli(format!("Failed to wait for gh: {e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            let error_msg = if stderr.is_empty() { stdout.to_string() } else { stderr.to_string() };
            return Err(classify_gh_error(&error_msg, account, args));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);

        // Handle empty output
        if stdout.trim().is_empty() {
            return Ok(Value::Null);
        }

        // Parse JSON output
        let json: Value = serde_json::from_str(&stdout)
            .map_err(|e| Error::GhCli(format!("Failed to parse gh output as JSON: {e}\nOutput: {stdout}")))?;

        Ok(json)
    }

    /// Run a gh command and return raw string output (for non-JSON commands like diff)
    pub async fn run_raw(&self, account: Option<&str>, args: &[&str]) -> Result<String> {
        let is_search = args.first().is_some_and(|a| *a == "search");

        // All requests go through the general limiter
        let general = self.get_limiter(account, false);
        if general.check().is_err() {
            tracing::debug!(
                account = account.unwrap_or("default"),
                command = %args.join(" "),
                "Rate limited - waiting for general quota"
            );
        }
        general.until_ready().await;

        // Search requests additionally go through the search limiter
        if is_search {
            let search = self.get_limiter(account, true);
            if search.check().is_err() {
                tracing::debug!(
                    account = account.unwrap_or("default"),
                    command = %args.join(" "),
                    "Rate limited - waiting for search quota"
                );
            }
            search.until_ready().await;
        }

        let token = self.config.get_token(account)?;

        tracing::debug!(
            "Running gh command (raw) with account {:?}: gh {}",
            account.unwrap_or("default"),
            args.join(" ")
        );

        let output = Command::new("gh")
            .args(args)
            .env("GH_TOKEN", &token)
            .env("NO_COLOR", "1")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| Error::GhCli(format!("Failed to spawn gh: {e}")))?
            .wait_with_output()
            .await
            .map_err(|e| Error::GhCli(format!("Failed to wait for gh: {e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            let error_msg = if stderr.is_empty() { stdout.to_string() } else { stderr.to_string() };
            return Err(classify_gh_error(&error_msg, account, args));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Run a gh api command
    ///
    /// # Arguments
    /// * `account` - Optional account name
    /// * `endpoint` - API endpoint (e.g., "user", "repos/{owner}/{repo}")
    /// * `method` - HTTP method (GET, POST, PUT, DELETE, PATCH)
    /// * `fields` - Optional fields to send with the request
    pub async fn api(
        &self,
        account: Option<&str>,
        endpoint: &str,
        method: Option<&str>,
        fields: Option<&[(&str, &str)]>,
    ) -> Result<Value> {
        let mut args = vec!["api"];

        if let Some(m) = method {
            args.push("-X");
            args.push(m);
        }

        args.push(endpoint);

        // Build field arguments
        let field_args: Vec<String> = fields.unwrap_or(&[]).iter().map(|(k, v)| format!("{k}={v}")).collect();

        let field_refs: Vec<&str> = field_args.iter().flat_map(|f| ["-f", f.as_str()]).collect();

        args.extend(field_refs);

        self.run(account, &args).await
    }

    /// Check gh CLI version
    pub async fn version(&self) -> Result<String> {
        let output = Command::new("gh")
            .args(["--version"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| Error::GhCli(format!("Failed to spawn gh: {e}")))?
            .wait_with_output()
            .await
            .map_err(|e| Error::GhCli(format!("Failed to wait for gh: {e}")))?;

        if !output.status.success() {
            return Err(Error::GhCli("Failed to get gh version".to_string()));
        }

        let version = String::from_utf8_lossy(&output.stdout)
            .lines()
            .next()
            .unwrap_or("unknown")
            .to_string();

        Ok(version)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{LogConfig, RateLimitConfig};
    use std::collections::HashMap;

    fn mock_config() -> Config {
        // This creates a config that won't have valid tokens
        // Real integration tests would need actual tokens
        let mut accounts = HashMap::new();
        accounts.insert("test".to_string(), "/nonexistent/path".to_string());
        Config {
            default_account: "test".to_string(),
            accounts,
            logging: LogConfig::default(),
            rate_limit: RateLimitConfig::default(),
        }
    }

    #[test]
    fn test_gh_client_creation() {
        // This test will pass if gh is installed
        let config = mock_config();
        let result = GhClient::new(config);

        // gh should be installed on this system
        if which::which("gh").is_ok() {
            assert!(result.is_ok());
        } else {
            assert!(matches!(result, Err(Error::GhNotFound)));
        }
    }

    #[tokio::test]
    async fn test_gh_version() {
        let config = mock_config();
        if let Ok(client) = GhClient::new(config) {
            let version = client.version().await;
            assert!(version.is_ok());
            assert!(version.unwrap().contains("gh version"));
        }
    }

    #[test]
    fn test_extract_rate_limit_reset_with_timestamp() {
        let msg = "HTTP 403: API rate limit exceeded - timestamp 2026-03-10 03:06:38 UTC";
        assert_eq!(extract_rate_limit_reset(msg), "2026-03-10 03:06:38 UTC");
    }

    #[test]
    fn test_extract_rate_limit_reset_without_timestamp() {
        let msg = "HTTP 403: API rate limit exceeded";
        assert_eq!(extract_rate_limit_reset(msg), "unknown");
    }

    #[test]
    fn test_classify_gh_error_rate_limit() {
        let msg = "HTTP 403: API rate limit exceeded - timestamp 2026-03-10 03:06:38 UTC";
        let err = classify_gh_error(msg, Some("home"), &["api", "user"]);
        match err {
            Error::RateLimit { account, reset_at } => {
                assert_eq!(account, "home");
                assert_eq!(reset_at, "2026-03-10 03:06:38 UTC");
            }
            other => panic!("Expected RateLimit, got: {other}"),
        }
    }

    #[test]
    fn test_classify_gh_error_secondary_rate_limit() {
        let msg = "You have exceeded a secondary rate limit";
        let err = classify_gh_error(msg, Some("work"), &["pr", "list"]);
        assert!(matches!(err, Error::RateLimit { .. }));
    }

    #[test]
    fn test_classify_gh_error_missing_scope() {
        let msg = "HTTP 403: missing_scope - requires 'repo' scope";
        let err = classify_gh_error(msg, Some("home"), &["repo", "create", "test"]);
        match err {
            Error::GhCli(msg) => {
                assert!(msg.contains("Missing OAuth scope"));
                assert!(msg.contains("home"));
                assert!(msg.contains("repo create test"));
            }
            other => panic!("Expected GhCli with scope info, got: {other}"),
        }
    }

    #[test]
    fn test_classify_gh_error_insufficient_scope() {
        let msg = "insufficient_scope error";
        let err = classify_gh_error(msg, None, &["api", "user"]);
        match err {
            Error::GhCli(msg) => {
                assert!(msg.contains("Missing OAuth scope"));
                assert!(msg.contains("default"));
            }
            other => panic!("Expected GhCli with scope info, got: {other}"),
        }
    }

    #[test]
    fn test_classify_gh_error_generic() {
        let msg = "  repository not found  ";
        let err = classify_gh_error(msg, Some("home"), &["repo", "view"]);
        match err {
            Error::GhCli(msg) => assert_eq!(msg, "repository not found"),
            other => panic!("Expected GhCli, got: {other}"),
        }
    }

    #[test]
    fn test_make_limiter_does_not_panic_with_zero() {
        let limiter = make_limiter(0);
        assert!(limiter.check().is_ok());
    }

    #[test]
    fn test_make_limiter_normal_value() {
        let limiter = make_limiter(80);
        assert!(limiter.check().is_ok());
    }

    #[test]
    fn test_get_limiter_same_account_returns_same_limiter() {
        if let Ok(client) = GhClient::new(mock_config()) {
            let l1 = client.get_limiter(Some("test"), false);
            let l2 = client.get_limiter(Some("test"), false);
            assert!(Arc::ptr_eq(&l1, &l2));
        }
    }

    #[test]
    fn test_get_limiter_different_accounts_return_different_limiters() {
        if let Ok(client) = GhClient::new(mock_config()) {
            let l1 = client.get_limiter(Some("account-a"), false);
            let l2 = client.get_limiter(Some("account-b"), false);
            assert!(!Arc::ptr_eq(&l1, &l2));
        }
    }

    #[test]
    fn test_get_limiter_search_vs_general_different() {
        if let Ok(client) = GhClient::new(mock_config()) {
            let general = client.get_limiter(Some("test"), false);
            let search = client.get_limiter(Some("test"), true);
            assert!(!Arc::ptr_eq(&general, &search));
        }
    }
}
