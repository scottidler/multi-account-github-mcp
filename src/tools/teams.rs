//! Teams and collaborator tool request types

use rmcp::schemars;
use schemars::JsonSchema;
use serde::Deserialize;

/// Request parameters for list_collaborators tool
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListCollaboratorsRequest {
    /// The account to use (e.g., 'home', 'work'). Uses default if not specified.
    #[schemars(description = "The account to use (e.g., 'home', 'work'). Uses default if not specified.")]
    pub account: Option<String>,

    /// Repository owner
    #[schemars(description = "Repository owner (user or organization)")]
    pub owner: String,

    /// Repository name
    #[schemars(description = "Repository name")]
    pub repo: String,

    /// Filter by affiliation (outside, direct, all)
    #[schemars(description = "Filter by affiliation: outside, direct, all (default: all)")]
    pub affiliation: Option<String>,
}

/// Request parameters for add_collaborator tool
#[derive(Debug, Deserialize, JsonSchema)]
pub struct AddCollaboratorRequest {
    /// The account to use (e.g., 'home', 'work'). Uses default if not specified.
    #[schemars(description = "The account to use (e.g., 'home', 'work'). Uses default if not specified.")]
    pub account: Option<String>,

    /// Repository owner
    #[schemars(description = "Repository owner (user or organization)")]
    pub owner: String,

    /// Repository name
    #[schemars(description = "Repository name")]
    pub repo: String,

    /// Username to add as collaborator
    #[schemars(description = "GitHub username to add as collaborator")]
    pub username: String,

    /// Permission level: pull, push, admin, maintain, triage
    #[schemars(description = "Permission level: pull, push, admin, maintain, triage (default: push)")]
    pub permission: Option<String>,
}

/// Request parameters for remove_collaborator tool
#[derive(Debug, Deserialize, JsonSchema)]
pub struct RemoveCollaboratorRequest {
    /// The account to use (e.g., 'home', 'work'). Uses default if not specified.
    #[schemars(description = "The account to use (e.g., 'home', 'work'). Uses default if not specified.")]
    pub account: Option<String>,

    /// Repository owner
    #[schemars(description = "Repository owner (user or organization)")]
    pub owner: String,

    /// Repository name
    #[schemars(description = "Repository name")]
    pub repo: String,

    /// Username to remove from collaborators
    #[schemars(description = "GitHub username to remove from collaborators")]
    pub username: String,
}

/// Request parameters for list_teams tool
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListTeamsRequest {
    /// The account to use (e.g., 'home', 'work'). Uses default if not specified.
    #[schemars(description = "The account to use (e.g., 'home', 'work'). Uses default if not specified.")]
    pub account: Option<String>,

    /// Organization name
    #[schemars(description = "Organization name")]
    pub org: String,

    /// Maximum number of teams to return
    #[schemars(description = "Maximum number of teams to return (default: 30)")]
    pub limit: Option<u32>,
}

/// Request parameters for get_team_members tool
#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetTeamMembersRequest {
    /// The account to use (e.g., 'home', 'work'). Uses default if not specified.
    #[schemars(description = "The account to use (e.g., 'home', 'work'). Uses default if not specified.")]
    pub account: Option<String>,

    /// Organization name
    #[schemars(description = "Organization name")]
    pub org: String,

    /// Team slug
    #[schemars(description = "Team slug (the URL-friendly name of the team)")]
    pub team: String,

    /// Filter by role: member, maintainer, all
    #[schemars(description = "Filter by role: member, maintainer, all (default: all)")]
    pub role: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_collaborators_request() {
        let json = r#"{"owner": "scottidler", "repo": "gx"}"#;
        let request: ListCollaboratorsRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.owner, "scottidler");
    }

    #[test]
    fn test_add_collaborator_request() {
        let json = r#"{"owner": "scottidler", "repo": "gx", "username": "newuser", "permission": "push"}"#;
        let request: AddCollaboratorRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.username, "newuser");
        assert_eq!(request.permission, Some("push".to_string()));
    }

    #[test]
    fn test_list_teams_request() {
        let json = r#"{"org": "tatari-tv"}"#;
        let request: ListTeamsRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.org, "tatari-tv");
    }
}
