//! Branch protection tool request types

use rmcp::schemars;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Request parameters for get_branch_protection tool
#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetBranchProtectionRequest {
    /// The account to use (e.g., 'home', 'work'). Uses default if not specified.
    #[schemars(description = "The account to use (e.g., 'home', 'work'). Uses default if not specified.")]
    pub account: Option<String>,

    /// Repository owner
    #[schemars(description = "Repository owner (user or organization)")]
    pub owner: String,

    /// Repository name
    #[schemars(description = "Repository name")]
    pub repo: String,

    /// Branch name
    #[schemars(description = "Branch name to get protection rules for")]
    pub branch: String,
}

/// Required status checks configuration
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct RequiredStatusChecks {
    /// Require branches to be up to date before merging
    #[schemars(description = "Require branches to be up to date before merging")]
    pub strict: Option<bool>,

    /// List of status check contexts that must pass
    #[schemars(description = "List of status check contexts that must pass")]
    pub contexts: Option<Vec<String>>,
}

/// Required pull request reviews configuration
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct RequiredPullRequestReviews {
    /// Number of required approving reviews
    #[schemars(description = "Number of required approving reviews")]
    pub required_approving_review_count: Option<u32>,

    /// Dismiss stale reviews when new commits are pushed
    #[schemars(description = "Dismiss stale reviews when new commits are pushed")]
    pub dismiss_stale_reviews: Option<bool>,

    /// Require review from code owners
    #[schemars(description = "Require review from code owners")]
    pub require_code_owner_reviews: Option<bool>,
}

/// Request parameters for set_branch_protection tool
#[derive(Debug, Deserialize, JsonSchema)]
pub struct SetBranchProtectionRequest {
    /// The account to use (e.g., 'home', 'work'). Uses default if not specified.
    #[schemars(description = "The account to use (e.g., 'home', 'work'). Uses default if not specified.")]
    pub account: Option<String>,

    /// Repository owner
    #[schemars(description = "Repository owner (user or organization)")]
    pub owner: String,

    /// Repository name
    #[schemars(description = "Repository name")]
    pub repo: String,

    /// Branch name
    #[schemars(description = "Branch name to set protection rules for")]
    pub branch: String,

    /// Require status checks to pass before merging
    #[schemars(description = "Require status checks to pass before merging")]
    pub required_status_checks: Option<RequiredStatusChecks>,

    /// Enforce all configured restrictions for administrators
    #[schemars(description = "Enforce all configured restrictions for administrators")]
    pub enforce_admins: Option<bool>,

    /// Require pull request reviews before merging
    #[schemars(description = "Require pull request reviews before merging")]
    pub required_pull_request_reviews: Option<RequiredPullRequestReviews>,

    /// Restrict who can push to the protected branch
    #[schemars(description = "Restrict who can push to the protected branch")]
    pub restrictions: Option<bool>,

    /// Require signed commits
    #[schemars(description = "Require signed commits")]
    pub required_signatures: Option<bool>,

    /// Require linear history
    #[schemars(description = "Require linear history (no merge commits)")]
    pub required_linear_history: Option<bool>,

    /// Allow force pushes
    #[schemars(description = "Allow force pushes")]
    pub allow_force_pushes: Option<bool>,

    /// Allow deletions
    #[schemars(description = "Allow branch deletions")]
    pub allow_deletions: Option<bool>,
}

/// Request parameters for delete_branch_protection tool
#[derive(Debug, Deserialize, JsonSchema)]
pub struct DeleteBranchProtectionRequest {
    /// The account to use (e.g., 'home', 'work'). Uses default if not specified.
    #[schemars(description = "The account to use (e.g., 'home', 'work'). Uses default if not specified.")]
    pub account: Option<String>,

    /// Repository owner
    #[schemars(description = "Repository owner (user or organization)")]
    pub owner: String,

    /// Repository name
    #[schemars(description = "Repository name")]
    pub repo: String,

    /// Branch name
    #[schemars(description = "Branch name to remove protection from")]
    pub branch: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_branch_protection_request() {
        let json = r#"{"owner": "scottidler", "repo": "gx", "branch": "main"}"#;
        let request: GetBranchProtectionRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.branch, "main");
    }

    #[test]
    fn test_set_branch_protection_request() {
        let json = r#"{
            "owner": "scottidler",
            "repo": "gx",
            "branch": "main",
            "enforce_admins": true,
            "required_pull_request_reviews": {
                "required_approving_review_count": 1
            }
        }"#;
        let request: SetBranchProtectionRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.enforce_admins, Some(true));
        assert!(request.required_pull_request_reviews.is_some());
    }
}
