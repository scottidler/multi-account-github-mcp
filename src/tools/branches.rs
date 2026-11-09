//! Branch-related tool request types

use rmcp::schemars;
use schemars::JsonSchema;
use serde::Deserialize;

/// Request parameters for list_branches tool
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListBranchesRequest {
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

/// Request parameters for create_branch tool
#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreateBranchRequest {
    /// The account to use (e.g., 'home', 'work'). Uses default if not specified.
    #[schemars(description = "The account to use (e.g., 'home', 'work'). Uses default if not specified.")]
    pub account: Option<String>,

    /// Repository owner
    #[schemars(description = "Repository owner (user or organization)")]
    pub owner: String,

    /// Repository name
    #[schemars(description = "Repository name")]
    pub repo: String,

    /// Name of the new branch
    #[schemars(description = "Name of the new branch to create")]
    pub branch: String,

    /// Source branch or commit SHA to branch from (default: default branch)
    #[schemars(description = "Source branch or commit SHA to branch from (default: default branch)")]
    pub from: Option<String>,
}

/// Request parameters for delete_branch tool
#[derive(Debug, Deserialize, JsonSchema)]
pub struct DeleteBranchRequest {
    /// The account to use (e.g., 'home', 'work'). Uses default if not specified.
    #[schemars(description = "The account to use (e.g., 'home', 'work'). Uses default if not specified.")]
    pub account: Option<String>,

    /// Repository owner
    #[schemars(description = "Repository owner (user or organization)")]
    pub owner: String,

    /// Repository name
    #[schemars(description = "Repository name")]
    pub repo: String,

    /// Name of the branch to delete
    #[schemars(description = "Name of the branch to delete")]
    pub branch: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_branch_request() {
        let json = r#"{"owner": "scottidler", "repo": "gx", "branch": "feature/test"}"#;
        let request: CreateBranchRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.owner, "scottidler");
        assert_eq!(request.repo, "gx");
        assert_eq!(request.branch, "feature/test");
    }

    #[test]
    fn test_delete_branch_request() {
        let json = r#"{"owner": "scottidler", "repo": "gx", "branch": "feature/test"}"#;
        let request: DeleteBranchRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.branch, "feature/test");
    }
}
