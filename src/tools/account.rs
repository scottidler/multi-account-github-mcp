//! Account-related tools (get_me)

use rmcp::schemars;
use schemars::JsonSchema;
use serde::Deserialize;

/// Request parameters for get_me tool
#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetMeRequest {
    /// The account to use (e.g., "home", "work"). Uses default account if not specified.
    #[schemars(description = "The account to use (e.g., 'home', 'work'). Uses default if not specified.")]
    pub account: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_me_request_deserialization() {
        let json = r#"{"account": "home"}"#;
        let request: GetMeRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.account, Some("home".to_string()));
    }

    #[test]
    fn test_get_me_request_empty() {
        let json = r#"{}"#;
        let request: GetMeRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.account, None);
    }
}
