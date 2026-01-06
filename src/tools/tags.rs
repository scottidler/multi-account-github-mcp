//! Tag tool request types

use rmcp::schemars;
use schemars::JsonSchema;
use serde::Deserialize;

/// Request parameters for list_tags tool
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListTagsRequest {
    /// The account to use (e.g., 'home', 'work'). Uses default if not specified.
    #[schemars(description = "The account to use (e.g., 'home', 'work'). Uses default if not specified.")]
    pub account: Option<String>,

    /// Repository owner
    #[schemars(description = "Repository owner (user or organization)")]
    pub owner: String,

    /// Repository name
    #[schemars(description = "Repository name")]
    pub repo: String,

    /// Maximum number of tags to return
    #[schemars(description = "Maximum number of tags to return (default: 30)")]
    pub limit: Option<u32>,
}

/// Request parameters for create_tag tool
#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreateTagRequest {
    /// The account to use (e.g., 'home', 'work'). Uses default if not specified.
    #[schemars(description = "The account to use (e.g., 'home', 'work'). Uses default if not specified.")]
    pub account: Option<String>,

    /// Repository owner
    #[schemars(description = "Repository owner (user or organization)")]
    pub owner: String,

    /// Repository name
    #[schemars(description = "Repository name")]
    pub repo: String,

    /// Tag name (e.g., 'v1.0.0')
    #[schemars(description = "Tag name (e.g., 'v1.0.0')")]
    pub tag: String,

    /// Commit SHA to tag
    #[schemars(description = "Commit SHA to tag (default: HEAD of default branch)")]
    pub sha: Option<String>,

    /// Tag message (creates annotated tag if provided)
    #[schemars(description = "Tag message (creates annotated tag if provided)")]
    pub message: Option<String>,
}

/// Request parameters for delete_tag tool
#[derive(Debug, Deserialize, JsonSchema)]
pub struct DeleteTagRequest {
    /// The account to use (e.g., 'home', 'work'). Uses default if not specified.
    #[schemars(description = "The account to use (e.g., 'home', 'work'). Uses default if not specified.")]
    pub account: Option<String>,

    /// Repository owner
    #[schemars(description = "Repository owner (user or organization)")]
    pub owner: String,

    /// Repository name
    #[schemars(description = "Repository name")]
    pub repo: String,

    /// Tag name to delete
    #[schemars(description = "Tag name to delete")]
    pub tag: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_tags_request() {
        let json = r#"{"owner": "scottidler", "repo": "gx"}"#;
        let request: ListTagsRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.owner, "scottidler");
    }

    #[test]
    fn test_create_tag_request() {
        let json = r#"{"owner": "scottidler", "repo": "gx", "tag": "v1.0.0", "message": "Release version 1.0.0"}"#;
        let request: CreateTagRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.tag, "v1.0.0");
        assert_eq!(request.message, Some("Release version 1.0.0".to_string()));
    }
}
