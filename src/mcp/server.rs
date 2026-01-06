//! MCP server implementation for GitHub multi-account

use crate::GhClient;
use crate::tools::account::GetMeRequest;
use rmcp::handler::server::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, Content, ServerInfo};
use rmcp::{ErrorData as McpError, ServerHandler, tool, tool_router};

/// GitHub MCP server with multi-account support
#[derive(Clone)]
pub struct GitHubMcpServer {
    gh: GhClient,
    #[allow(dead_code)] // Used by rmcp tool_router macro
    tool_router: ToolRouter<Self>,
}

impl GitHubMcpServer {
    /// Create a new GitHub MCP server
    pub fn new(gh: GhClient) -> Self {
        Self {
            gh,
            tool_router: Self::tool_router(),
        }
    }
}

#[tool_router]
impl GitHubMcpServer {
    /// Get information about the authenticated GitHub user
    #[tool(
        description = "Get the authenticated GitHub user's information. Use the 'account' parameter to specify which account to use (e.g., 'home', 'work'). If not specified, the default account will be used."
    )]
    async fn get_me(&self, params: Parameters<GetMeRequest>) -> Result<CallToolResult, McpError> {
        let result = self
            .gh
            .run(params.0.account.as_deref(), &["api", "user"])
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        Ok(CallToolResult::success(vec![Content::json(&result)?]))
    }
}

impl ServerHandler for GitHubMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "GitHub MCP server with multi-account support. \
                 Use the 'account' parameter to specify which GitHub account to use (e.g., 'home' or 'work'). \
                 If not specified, the default account will be used."
                    .to_string(),
            ),
            ..Default::default()
        }
    }
}
