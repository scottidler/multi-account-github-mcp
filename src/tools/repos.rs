//! Repository-related tool request types

use rmcp::schemars;
use schemars::JsonSchema;
use serde::Deserialize;

/// Request parameters for create_repo tool
#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreateRepoRequest {
    /// The account to use (e.g., 'home', 'work'). Uses default if not specified.
    #[schemars(description = "The account to use (e.g., 'home', 'work'). Uses default if not specified.")]
    pub account: Option<String>,

    /// Name of the repository
    #[schemars(description = "Name of the repository to create")]
    pub name: String,

    /// Description of the repository
    #[schemars(description = "Description of the repository")]
    pub description: Option<String>,

    /// Whether the repository should be private
    #[schemars(description = "Whether the repository should be private (default: false)")]
    pub private: Option<bool>,

    /// Organization to create the repo in (omit for personal repo)
    #[schemars(description = "Organization to create the repo in (omit for personal repo)")]
    pub org: Option<String>,
}

/// Request parameters for list_repos tool
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListReposRequest {
    /// The account to use (e.g., 'home', 'work'). Uses default if not specified.
    #[schemars(description = "The account to use (e.g., 'home', 'work'). Uses default if not specified.")]
    pub account: Option<String>,

    /// Owner (user or org) to list repos for. Defaults to authenticated user.
    #[schemars(description = "Owner (user or org) to list repos for. Defaults to authenticated user.")]
    pub owner: Option<String>,

    /// Maximum number of repos to return
    #[schemars(description = "Maximum number of repos to return (default: 30)")]
    pub limit: Option<u32>,
}

/// Request parameters for get_repo tool
#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetRepoRequest {
    /// The account to use (e.g., 'home', 'work'). Uses default if not specified.
    #[schemars(description = "The account to use (e.g., 'home', 'work'). Uses default if not specified.")]
    pub account: Option<String>,

    /// Repository owner
    #[schemars(description = "Repository owner (user or organization)")]
    pub owner: String,

    /// Repository name
    #[schemars(description = "Repository name")]
    pub repo: String,
}

/// Request parameters for archive_repo tool
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ArchiveRepoRequest {
    /// The account to use (e.g., 'home', 'work'). Uses default if not specified.
    #[schemars(description = "The account to use (e.g., 'home', 'work'). Uses default if not specified.")]
    pub account: Option<String>,

    /// Repository owner
    #[schemars(description = "Repository owner (user or organization)")]
    pub owner: String,

    /// Repository name
    #[schemars(description = "Repository name")]
    pub repo: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_repo_request() {
        let json = r#"{"name": "test-repo", "private": true}"#;
        let request: CreateRepoRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.name, "test-repo");
        assert_eq!(request.private, Some(true));
    }

    #[test]
    fn test_get_repo_request() {
        let json = r#"{"owner": "scottidler", "repo": "gx"}"#;
        let request: GetRepoRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.owner, "scottidler");
        assert_eq!(request.repo, "gx");
    }
}
