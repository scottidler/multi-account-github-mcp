//! gh CLI wrapper for multi-account-github-mcp

use crate::{Config, Error, Result};
use serde_json::Value;
use std::process::Stdio;
use std::sync::Arc;
use tokio::process::Command;

/// Client for executing gh CLI commands with account-specific tokens
#[derive(Debug, Clone)]
pub struct GhClient {
    config: Arc<Config>,
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
        })
    }

    /// Get the underlying config
    pub fn config(&self) -> &Config {
        &self.config
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
            return Err(Error::GhCli(error_msg.trim().to_string()));
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
            return Err(Error::GhCli(error_msg.trim().to_string()));
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
    use std::collections::HashMap;

    fn mock_config() -> Config {
        // This creates a config that won't have valid tokens
        // Real integration tests would need actual tokens
        let mut accounts = HashMap::new();
        accounts.insert("test".to_string(), "/nonexistent/path".to_string());
        Config {
            default_account: "test".to_string(),
            accounts,
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
}
