//! Pull request tool request types

use rmcp::schemars;
use schemars::JsonSchema;
use serde::Deserialize;

/// Request parameters for get_pr tool
#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetPrRequest {
    /// The account to use (e.g., 'home', 'work'). Uses default if not specified.
    #[schemars(description = "The account to use (e.g., 'home', 'work'). Uses default if not specified.")]
    pub account: Option<String>,

    /// Repository owner
    #[schemars(description = "Repository owner (user or organization)")]
    pub owner: String,

    /// Repository name
    #[schemars(description = "Repository name")]
    pub repo: String,

    /// Pull request number
    #[schemars(description = "Pull request number")]
    pub number: u64,
}

/// Request parameters for get_pr_diff tool
#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetPrDiffRequest {
    /// The account to use (e.g., 'home', 'work'). Uses default if not specified.
    #[schemars(description = "The account to use (e.g., 'home', 'work'). Uses default if not specified.")]
    pub account: Option<String>,

    /// Repository owner
    #[schemars(description = "Repository owner (user or organization)")]
    pub owner: String,

    /// Repository name
    #[schemars(description = "Repository name")]
    pub repo: String,

    /// Pull request number
    #[schemars(description = "Pull request number")]
    pub number: u64,
}

/// Request parameters for get_pr_files tool
#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetPrFilesRequest {
    /// The account to use (e.g., 'home', 'work'). Uses default if not specified.
    #[schemars(description = "The account to use (e.g., 'home', 'work'). Uses default if not specified.")]
    pub account: Option<String>,

    /// Repository owner
    #[schemars(description = "Repository owner (user or organization)")]
    pub owner: String,

    /// Repository name
    #[schemars(description = "Repository name")]
    pub repo: String,

    /// Pull request number
    #[schemars(description = "Pull request number")]
    pub number: u64,
}

/// Request parameters for list_prs tool
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListPrsRequest {
    /// The account to use (e.g., 'home', 'work'). Uses default if not specified.
    #[schemars(description = "The account to use (e.g., 'home', 'work'). Uses default if not specified.")]
    pub account: Option<String>,

    /// Repository owner
    #[schemars(description = "Repository owner (user or organization)")]
    pub owner: String,

    /// Repository name
    #[schemars(description = "Repository name")]
    pub repo: String,

    /// Filter by state: open, closed, merged, all
    #[schemars(description = "Filter by state: open, closed, merged, all (default: open)")]
    pub state: Option<String>,

    /// Maximum number of PRs to return
    #[schemars(description = "Maximum number of PRs to return (default: 30)")]
    pub limit: Option<u32>,

    /// Filter by base branch
    #[schemars(description = "Filter by base branch")]
    pub base: Option<String>,

    /// Filter by head branch
    #[schemars(description = "Filter by head branch")]
    pub head: Option<String>,
}

/// Request parameters for search_prs tool
#[derive(Debug, Deserialize, JsonSchema)]
pub struct SearchPrsRequest {
    /// The account to use (e.g., 'home', 'work'). Uses default if not specified.
    #[schemars(description = "The account to use (e.g., 'home', 'work'). Uses default if not specified.")]
    pub account: Option<String>,

    /// Search query (GitHub search syntax)
    #[schemars(description = "Search query using GitHub search syntax")]
    pub query: String,

    /// Maximum number of results
    #[schemars(description = "Maximum number of results (default: 30)")]
    pub limit: Option<u32>,
}

/// Request parameters for create_pr tool
#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreatePrRequest {
    /// The account to use (e.g., 'home', 'work'). Uses default if not specified.
    #[schemars(description = "The account to use (e.g., 'home', 'work'). Uses default if not specified.")]
    pub account: Option<String>,

    /// Repository owner
    #[schemars(description = "Repository owner (user or organization)")]
    pub owner: String,

    /// Repository name
    #[schemars(description = "Repository name")]
    pub repo: String,

    /// PR title
    #[schemars(description = "Pull request title")]
    pub title: String,

    /// PR body/description
    #[schemars(description = "Pull request body/description")]
    pub body: Option<String>,

    /// Head branch (the branch with changes)
    #[schemars(description = "Head branch containing the changes")]
    pub head: String,

    /// Base branch (the branch to merge into)
    #[schemars(description = "Base branch to merge into (default: default branch)")]
    pub base: Option<String>,

    /// Create as draft PR
    #[schemars(description = "Create as draft PR")]
    pub draft: Option<bool>,
}

/// Request parameters for edit_pr tool
#[derive(Debug, Deserialize, JsonSchema)]
pub struct EditPrRequest {
    /// The account to use (e.g., 'home', 'work'). Uses default if not specified.
    #[schemars(description = "The account to use (e.g., 'home', 'work'). Uses default if not specified.")]
    pub account: Option<String>,

    /// Repository owner
    #[schemars(description = "Repository owner (user or organization)")]
    pub owner: String,

    /// Repository name
    #[schemars(description = "Repository name")]
    pub repo: String,

    /// Pull request number
    #[schemars(description = "Pull request number")]
    pub number: u64,

    /// New title
    #[schemars(description = "New title for the PR")]
    pub title: Option<String>,

    /// New body/description
    #[schemars(description = "New body/description for the PR")]
    pub body: Option<String>,

    /// New base branch
    #[schemars(description = "New base branch")]
    pub base: Option<String>,
}

/// Request parameters for merge_pr tool
#[derive(Debug, Deserialize, JsonSchema)]
pub struct MergePrRequest {
    /// The account to use (e.g., 'home', 'work'). Uses default if not specified.
    #[schemars(description = "The account to use (e.g., 'home', 'work'). Uses default if not specified.")]
    pub account: Option<String>,

    /// Repository owner
    #[schemars(description = "Repository owner (user or organization)")]
    pub owner: String,

    /// Repository name
    #[schemars(description = "Repository name")]
    pub repo: String,

    /// Pull request number
    #[schemars(description = "Pull request number")]
    pub number: u64,

    /// Merge method: merge, squash, rebase
    #[schemars(description = "Merge method: merge, squash, rebase (default: merge)")]
    pub method: Option<String>,

    /// Delete head branch after merge
    #[schemars(description = "Delete the head branch after merging")]
    pub delete_branch: Option<bool>,

    /// Custom commit message for squash/merge
    #[schemars(description = "Custom commit message")]
    pub commit_message: Option<String>,
}

/// Request parameters for close_pr tool
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ClosePrRequest {
    /// The account to use (e.g., 'home', 'work'). Uses default if not specified.
    #[schemars(description = "The account to use (e.g., 'home', 'work'). Uses default if not specified.")]
    pub account: Option<String>,

    /// Repository owner
    #[schemars(description = "Repository owner (user or organization)")]
    pub owner: String,

    /// Repository name
    #[schemars(description = "Repository name")]
    pub repo: String,

    /// Pull request number
    #[schemars(description = "Pull request number")]
    pub number: u64,
}

/// Request parameters for comment_pr tool
#[derive(Debug, Deserialize, JsonSchema)]
pub struct CommentPrRequest {
    /// The account to use (e.g., 'home', 'work'). Uses default if not specified.
    #[schemars(description = "The account to use (e.g., 'home', 'work'). Uses default if not specified.")]
    pub account: Option<String>,

    /// Repository owner
    #[schemars(description = "Repository owner (user or organization)")]
    pub owner: String,

    /// Repository name
    #[schemars(description = "Repository name")]
    pub repo: String,

    /// Pull request number
    #[schemars(description = "Pull request number")]
    pub number: u64,

    /// Comment body
    #[schemars(description = "Comment body (Markdown supported)")]
    pub body: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_pr_request() {
        let json = r#"{"owner": "scottidler", "repo": "gx", "number": 42}"#;
        let request: GetPrRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.number, 42);
    }

    #[test]
    fn test_create_pr_request() {
        let json = r#"{"owner": "scottidler", "repo": "gx", "title": "Add feature", "head": "feature-branch"}"#;
        let request: CreatePrRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.title, "Add feature");
        assert_eq!(request.head, "feature-branch");
    }

    #[test]
    fn test_merge_pr_request() {
        let json = r#"{"owner": "scottidler", "repo": "gx", "number": 42, "method": "squash", "delete_branch": true}"#;
        let request: MergePrRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.method, Some("squash".to_string()));
        assert_eq!(request.delete_branch, Some(true));
    }
}
