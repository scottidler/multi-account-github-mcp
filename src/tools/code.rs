//! Code and content tool request types

use rmcp::schemars;
use schemars::JsonSchema;
use serde::Deserialize;

/// Request parameters for get_file tool
#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetFileRequest {
    /// The account to use (e.g., 'home', 'work'). Uses default if not specified.
    #[schemars(description = "The account to use (e.g., 'home', 'work'). Uses default if not specified.")]
    pub account: Option<String>,

    /// Repository owner
    #[schemars(description = "Repository owner (user or organization)")]
    pub owner: String,

    /// Repository name
    #[schemars(description = "Repository name")]
    pub repo: String,

    /// File path within the repository
    #[schemars(description = "File path within the repository")]
    pub path: String,

    /// Git ref (branch, tag, or commit SHA)
    #[schemars(description = "Git ref (branch, tag, or commit SHA). Defaults to default branch.")]
    pub r#ref: Option<String>,
}

/// Request parameters for search_code tool
#[derive(Debug, Deserialize, JsonSchema)]
pub struct SearchCodeRequest {
    /// The account to use (e.g., 'home', 'work'). Uses default if not specified.
    #[schemars(description = "The account to use (e.g., 'home', 'work'). Uses default if not specified.")]
    pub account: Option<String>,

    /// Search query (GitHub code search syntax)
    #[schemars(description = "Search query using GitHub code search syntax")]
    pub query: String,

    /// Maximum number of results
    #[schemars(description = "Maximum number of results (default: 30)")]
    pub limit: Option<u32>,
}

/// Request parameters for list_commits tool
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListCommitsRequest {
    /// The account to use (e.g., 'home', 'work'). Uses default if not specified.
    #[schemars(description = "The account to use (e.g., 'home', 'work'). Uses default if not specified.")]
    pub account: Option<String>,

    /// Repository owner
    #[schemars(description = "Repository owner (user or organization)")]
    pub owner: String,

    /// Repository name
    #[schemars(description = "Repository name")]
    pub repo: String,

    /// Branch or commit SHA to list commits from
    #[schemars(description = "Branch or commit SHA to list commits from")]
    pub sha: Option<String>,

    /// File path to filter commits by
    #[schemars(description = "File path to filter commits by")]
    pub path: Option<String>,

    /// Author to filter commits by
    #[schemars(description = "Author username or email to filter commits by")]
    pub author: Option<String>,

    /// Maximum number of commits to return
    #[schemars(description = "Maximum number of commits to return (default: 30)")]
    pub limit: Option<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_file_request() {
        let json = r#"{"owner": "scottidler", "repo": "gx", "path": "README.md"}"#;
        let request: GetFileRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.path, "README.md");
    }

    #[test]
    fn test_search_code_request() {
        let json = r#"{"query": "function test language:rust"}"#;
        let request: SearchCodeRequest = serde_json::from_str(json).unwrap();
        assert!(request.query.contains("function test"));
    }

    #[test]
    fn test_list_commits_request() {
        let json = r#"{"owner": "scottidler", "repo": "gx", "sha": "main", "limit": 10}"#;
        let request: ListCommitsRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.sha, Some("main".to_string()));
        assert_eq!(request.limit, Some(10));
    }
}
