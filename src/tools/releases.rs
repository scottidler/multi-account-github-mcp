//! Release tool request types

use rmcp::schemars;
use schemars::JsonSchema;
use serde::Deserialize;

/// Request parameters for list_releases tool
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListReleasesRequest {
    /// The account to use (e.g., 'home', 'work'). Uses default if not specified.
    #[schemars(description = "The account to use (e.g., 'home', 'work'). Uses default if not specified.")]
    pub account: Option<String>,

    /// Repository owner
    #[schemars(description = "Repository owner (user or organization)")]
    pub owner: String,

    /// Repository name
    #[schemars(description = "Repository name")]
    pub repo: String,

    /// Maximum number of releases to return
    #[schemars(description = "Maximum number of releases to return (default: 30)")]
    pub limit: Option<u32>,
}

/// Request parameters for get_release tool
#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetReleaseRequest {
    /// The account to use (e.g., 'home', 'work'). Uses default if not specified.
    #[schemars(description = "The account to use (e.g., 'home', 'work'). Uses default if not specified.")]
    pub account: Option<String>,

    /// Repository owner
    #[schemars(description = "Repository owner (user or organization)")]
    pub owner: String,

    /// Repository name
    #[schemars(description = "Repository name")]
    pub repo: String,

    /// Release tag (e.g., 'v1.0.0')
    #[schemars(description = "Release tag (e.g., 'v1.0.0')")]
    pub tag: String,
}

/// Request parameters for create_release tool
#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreateReleaseRequest {
    /// The account to use (e.g., 'home', 'work'). Uses default if not specified.
    #[schemars(description = "The account to use (e.g., 'home', 'work'). Uses default if not specified.")]
    pub account: Option<String>,

    /// Repository owner
    #[schemars(description = "Repository owner (user or organization)")]
    pub owner: String,

    /// Repository name
    #[schemars(description = "Repository name")]
    pub repo: String,

    /// Tag name for the release (e.g., 'v1.0.0')
    #[schemars(description = "Tag name for the release (e.g., 'v1.0.0')")]
    pub tag: String,

    /// Release title
    #[schemars(description = "Release title")]
    pub title: Option<String>,

    /// Release notes/body
    #[schemars(description = "Release notes/body (Markdown supported)")]
    pub notes: Option<String>,

    /// Target commit SHA or branch (default: default branch)
    #[schemars(description = "Target commit SHA or branch (default: default branch)")]
    pub target: Option<String>,

    /// Create as draft release
    #[schemars(description = "Create as draft release")]
    pub draft: Option<bool>,

    /// Mark as prerelease
    #[schemars(description = "Mark as prerelease")]
    pub prerelease: Option<bool>,

    /// Auto-generate release notes
    #[schemars(description = "Auto-generate release notes from commits")]
    pub generate_notes: Option<bool>,
}

/// Request parameters for delete_release tool
#[derive(Debug, Deserialize, JsonSchema)]
pub struct DeleteReleaseRequest {
    /// The account to use (e.g., 'home', 'work'). Uses default if not specified.
    #[schemars(description = "The account to use (e.g., 'home', 'work'). Uses default if not specified.")]
    pub account: Option<String>,

    /// Repository owner
    #[schemars(description = "Repository owner (user or organization)")]
    pub owner: String,

    /// Repository name
    #[schemars(description = "Repository name")]
    pub repo: String,

    /// Release tag to delete
    #[schemars(description = "Release tag to delete")]
    pub tag: String,

    /// Also delete the associated tag
    #[schemars(description = "Also delete the associated git tag")]
    pub delete_tag: Option<bool>,
}

/// Request parameters for list_release_assets tool
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListReleaseAssetsRequest {
    /// The account to use (e.g., 'home', 'work'). Uses default if not specified.
    #[schemars(description = "The account to use (e.g., 'home', 'work'). Uses default if not specified.")]
    pub account: Option<String>,

    /// Repository owner
    #[schemars(description = "Repository owner (user or organization)")]
    pub owner: String,

    /// Repository name
    #[schemars(description = "Repository name")]
    pub repo: String,

    /// Release tag
    #[schemars(description = "Release tag (e.g., 'v1.0.0')")]
    pub tag: String,
}

/// Request parameters for download_release_asset tool
#[derive(Debug, Deserialize, JsonSchema)]
pub struct DownloadReleaseAssetRequest {
    /// The account to use (e.g., 'home', 'work'). Uses default if not specified.
    #[schemars(description = "The account to use (e.g., 'home', 'work'). Uses default if not specified.")]
    pub account: Option<String>,

    /// Repository owner
    #[schemars(description = "Repository owner (user or organization)")]
    pub owner: String,

    /// Repository name
    #[schemars(description = "Repository name")]
    pub repo: String,

    /// Release tag
    #[schemars(description = "Release tag (e.g., 'v1.0.0')")]
    pub tag: String,

    /// Specific asset pattern to download (glob pattern)
    #[schemars(description = "Asset pattern to download (glob pattern, e.g., '*.tar.gz')")]
    pub pattern: Option<String>,

    /// Directory to download to
    #[schemars(description = "Directory to download assets to (default: current directory)")]
    pub dir: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_releases_request() {
        let json = r#"{"owner": "scottidler", "repo": "gx"}"#;
        let request: ListReleasesRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.owner, "scottidler");
    }

    #[test]
    fn test_create_release_request() {
        let json =
            r#"{"owner": "scottidler", "repo": "gx", "tag": "v1.0.0", "title": "Release 1.0", "prerelease": false}"#;
        let request: CreateReleaseRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.tag, "v1.0.0");
        assert_eq!(request.prerelease, Some(false));
    }
}
