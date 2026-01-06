//! Workflow and artifact tool request types

use rmcp::schemars;
use schemars::JsonSchema;
use serde::Deserialize;

/// Request parameters for list_workflow_runs tool
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListWorkflowRunsRequest {
    /// The account to use (e.g., 'home', 'work'). Uses default if not specified.
    #[schemars(description = "The account to use (e.g., 'home', 'work'). Uses default if not specified.")]
    pub account: Option<String>,

    /// Repository owner
    #[schemars(description = "Repository owner (user or organization)")]
    pub owner: String,

    /// Repository name
    #[schemars(description = "Repository name")]
    pub repo: String,

    /// Filter by workflow name or file
    #[schemars(description = "Filter by workflow name or file (e.g., 'ci.yml')")]
    pub workflow: Option<String>,

    /// Filter by branch
    #[schemars(description = "Filter by branch name")]
    pub branch: Option<String>,

    /// Filter by status: queued, in_progress, completed
    #[schemars(description = "Filter by status: queued, in_progress, completed")]
    pub status: Option<String>,

    /// Maximum number of runs to return
    #[schemars(description = "Maximum number of runs to return (default: 20)")]
    pub limit: Option<u32>,
}

/// Request parameters for list_run_artifacts tool
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListRunArtifactsRequest {
    /// The account to use (e.g., 'home', 'work'). Uses default if not specified.
    #[schemars(description = "The account to use (e.g., 'home', 'work'). Uses default if not specified.")]
    pub account: Option<String>,

    /// Repository owner
    #[schemars(description = "Repository owner (user or organization)")]
    pub owner: String,

    /// Repository name
    #[schemars(description = "Repository name")]
    pub repo: String,

    /// Workflow run ID
    #[schemars(description = "Workflow run ID")]
    pub run_id: u64,
}

/// Request parameters for download_run_artifact tool
#[derive(Debug, Deserialize, JsonSchema)]
pub struct DownloadRunArtifactRequest {
    /// The account to use (e.g., 'home', 'work'). Uses default if not specified.
    #[schemars(description = "The account to use (e.g., 'home', 'work'). Uses default if not specified.")]
    pub account: Option<String>,

    /// Repository owner
    #[schemars(description = "Repository owner (user or organization)")]
    pub owner: String,

    /// Repository name
    #[schemars(description = "Repository name")]
    pub repo: String,

    /// Workflow run ID
    #[schemars(description = "Workflow run ID")]
    pub run_id: u64,

    /// Artifact name pattern to download
    #[schemars(description = "Artifact name pattern to download (e.g., 'build-*')")]
    pub name: Option<String>,

    /// Directory to download to
    #[schemars(description = "Directory to download artifacts to (default: current directory)")]
    pub dir: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_workflow_runs_request() {
        let json = r#"{"owner": "scottidler", "repo": "gx", "status": "completed"}"#;
        let request: ListWorkflowRunsRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.status, Some("completed".to_string()));
    }

    #[test]
    fn test_list_run_artifacts_request() {
        let json = r#"{"owner": "scottidler", "repo": "gx", "run_id": 123456789}"#;
        let request: ListRunArtifactsRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.run_id, 123456789);
    }
}
