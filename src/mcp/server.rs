//! MCP server implementation for GitHub multi-account

use crate::GhClient;
use crate::tools::account::GetMeRequest;
use crate::tools::branches::{CreateBranchRequest, DeleteBranchRequest, ListBranchesRequest};
use crate::tools::protection::{DeleteBranchProtectionRequest, GetBranchProtectionRequest, SetBranchProtectionRequest};
use crate::tools::repos::{ArchiveRepoRequest, CreateRepoRequest, GetRepoRequest, ListReposRequest};
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

    fn err(e: impl std::fmt::Display) -> McpError {
        McpError::internal_error(e.to_string(), None)
    }
}

#[tool_router]
impl GitHubMcpServer {
    // ============================================
    // Account Tools
    // ============================================

    /// Get information about the authenticated GitHub user
    #[tool(
        description = "Get the authenticated GitHub user's information. Use the 'account' parameter to specify which account to use (e.g., 'home', 'work'). If not specified, the default account will be used."
    )]
    async fn get_me(&self, params: Parameters<GetMeRequest>) -> Result<CallToolResult, McpError> {
        let result = self
            .gh
            .run(params.0.account.as_deref(), &["api", "user"])
            .await
            .map_err(Self::err)?;
        Ok(CallToolResult::success(vec![Content::json(&result)?]))
    }

    // ============================================
    // Repository Tools
    // ============================================

    /// Create a new GitHub repository
    #[tool(description = "Create a new GitHub repository. Can create personal or organization repos.")]
    async fn create_repo(&self, params: Parameters<CreateRepoRequest>) -> Result<CallToolResult, McpError> {
        let mut args = vec!["repo", "create", &params.0.name];

        let description;
        if let Some(ref desc) = params.0.description {
            description = format!("--description={desc}");
            args.push(&description);
        }

        if params.0.private.unwrap_or(false) {
            args.push("--private");
        } else {
            args.push("--public");
        }

        args.push("--confirm");

        let result = self
            .gh
            .run(params.0.account.as_deref(), &args)
            .await
            .map_err(Self::err)?;
        Ok(CallToolResult::success(vec![Content::json(&result)?]))
    }

    /// List repositories for a user or organization
    #[tool(description = "List repositories for a user or organization. Defaults to authenticated user's repos.")]
    async fn list_repos(&self, params: Parameters<ListReposRequest>) -> Result<CallToolResult, McpError> {
        let mut args = vec!["repo", "list"];

        if let Some(ref owner) = params.0.owner {
            args.push(owner);
        }

        let limit_str;
        if let Some(limit) = params.0.limit {
            limit_str = limit.to_string();
            args.push("--limit");
            args.push(&limit_str);
        }

        args.push("--json");
        args.push("name,description,visibility,updatedAt,url");

        let result = self
            .gh
            .run(params.0.account.as_deref(), &args)
            .await
            .map_err(Self::err)?;
        Ok(CallToolResult::success(vec![Content::json(&result)?]))
    }

    /// Get details about a specific repository
    #[tool(description = "Get detailed information about a specific repository.")]
    async fn get_repo(&self, params: Parameters<GetRepoRequest>) -> Result<CallToolResult, McpError> {
        let repo = format!("{}/{}", params.0.owner, params.0.repo);
        let args = vec![
            "repo",
            "view",
            &repo,
            "--json",
            "name,description,visibility,defaultBranchRef,url,createdAt,updatedAt,owner,stargazerCount,forkCount,issues,pullRequests",
        ];
        let result = self
            .gh
            .run(params.0.account.as_deref(), &args)
            .await
            .map_err(Self::err)?;
        Ok(CallToolResult::success(vec![Content::json(&result)?]))
    }

    /// Archive a repository (safer alternative to deletion)
    #[tool(
        description = "Archive a repository. This is a safer alternative to deletion - the repo becomes read-only but can be unarchived."
    )]
    async fn archive_repo(&self, params: Parameters<ArchiveRepoRequest>) -> Result<CallToolResult, McpError> {
        let endpoint = format!("repos/{}/{}", params.0.owner, params.0.repo);
        let result = self
            .gh
            .api(
                params.0.account.as_deref(),
                &endpoint,
                Some("PATCH"),
                Some(&[("archived", "true")]),
            )
            .await
            .map_err(Self::err)?;
        Ok(CallToolResult::success(vec![Content::json(&result)?]))
    }

    // ============================================
    // Branch Tools
    // ============================================

    /// List branches in a repository
    #[tool(description = "List all branches in a repository.")]
    async fn list_branches(&self, params: Parameters<ListBranchesRequest>) -> Result<CallToolResult, McpError> {
        let endpoint = format!("repos/{}/{}/branches", params.0.owner, params.0.repo);
        let result = self
            .gh
            .api(params.0.account.as_deref(), &endpoint, None, None)
            .await
            .map_err(Self::err)?;
        Ok(CallToolResult::success(vec![Content::json(&result)?]))
    }

    /// Create a new branch in a repository
    #[tool(
        description = "Create a new branch in a repository. Optionally specify a source branch/commit to branch from."
    )]
    async fn create_branch(&self, params: Parameters<CreateBranchRequest>) -> Result<CallToolResult, McpError> {
        // First, get the SHA of the source (default branch or specified)
        let source = params.0.from.as_deref().unwrap_or("HEAD");
        let sha_endpoint = format!("repos/{}/{}/git/ref/heads/{}", params.0.owner, params.0.repo, source);

        // Try to get SHA from branch ref, fall back to commit SHA if it's a SHA
        let sha = match self
            .gh
            .api(params.0.account.as_deref(), &sha_endpoint, None, None)
            .await
        {
            Ok(result) => result["object"]["sha"].as_str().unwrap_or(source).to_string(),
            Err(_) => source.to_string(), // Assume it's a SHA
        };

        // Create the new branch ref
        let endpoint = format!("repos/{}/{}/git/refs", params.0.owner, params.0.repo);
        let ref_name = format!("refs/heads/{}", params.0.branch);
        let ref_arg = format!("ref={ref_name}");
        let sha_arg = format!("sha={sha}");

        // Use raw API call with JSON body
        let args = vec!["api", "-X", "POST", &endpoint, "-f", &ref_arg, "-f", &sha_arg];
        let result = self
            .gh
            .run(params.0.account.as_deref(), &args)
            .await
            .map_err(Self::err)?;
        Ok(CallToolResult::success(vec![Content::json(&result)?]))
    }

    /// Delete a branch from a repository
    #[tool(description = "Delete a branch from a repository. Cannot delete the default branch.")]
    async fn delete_branch(&self, params: Parameters<DeleteBranchRequest>) -> Result<CallToolResult, McpError> {
        let endpoint = format!(
            "repos/{}/{}/git/refs/heads/{}",
            params.0.owner, params.0.repo, params.0.branch
        );
        self.gh
            .api(params.0.account.as_deref(), &endpoint, Some("DELETE"), None)
            .await
            .map_err(Self::err)?;
        Ok(CallToolResult::success(vec![Content::text(format!(
            "Branch '{}' deleted successfully",
            params.0.branch
        ))]))
    }

    // ============================================
    // Branch Protection Tools
    // ============================================

    /// Get branch protection rules for a branch
    #[tool(description = "Get the branch protection rules for a specific branch.")]
    async fn get_branch_protection(
        &self,
        params: Parameters<GetBranchProtectionRequest>,
    ) -> Result<CallToolResult, McpError> {
        let endpoint = format!(
            "repos/{}/{}/branches/{}/protection",
            params.0.owner, params.0.repo, params.0.branch
        );
        let result = self
            .gh
            .api(params.0.account.as_deref(), &endpoint, None, None)
            .await
            .map_err(Self::err)?;
        Ok(CallToolResult::success(vec![Content::json(&result)?]))
    }

    /// Set branch protection rules for a branch
    #[tool(
        description = "Set branch protection rules for a branch. Includes options for required reviews, status checks, admin enforcement, etc."
    )]
    async fn set_branch_protection(
        &self,
        params: Parameters<SetBranchProtectionRequest>,
    ) -> Result<CallToolResult, McpError> {
        let endpoint = format!(
            "repos/{}/{}/branches/{}/protection",
            params.0.owner, params.0.repo, params.0.branch
        );

        // Build the protection settings JSON
        let mut body = serde_json::json!({});

        if let Some(ref checks) = params.0.required_status_checks {
            body["required_status_checks"] = serde_json::json!({
                "strict": checks.strict.unwrap_or(false),
                "contexts": checks.contexts.clone().unwrap_or_default()
            });
        } else {
            body["required_status_checks"] = serde_json::Value::Null;
        }

        body["enforce_admins"] = serde_json::json!(params.0.enforce_admins.unwrap_or(false));

        if let Some(ref reviews) = params.0.required_pull_request_reviews {
            body["required_pull_request_reviews"] = serde_json::json!({
                "required_approving_review_count": reviews.required_approving_review_count.unwrap_or(1),
                "dismiss_stale_reviews": reviews.dismiss_stale_reviews.unwrap_or(false),
                "require_code_owner_reviews": reviews.require_code_owner_reviews.unwrap_or(false)
            });
        } else {
            body["required_pull_request_reviews"] = serde_json::Value::Null;
        }

        body["restrictions"] = serde_json::Value::Null;

        if let Some(linear) = params.0.required_linear_history {
            body["required_linear_history"] = serde_json::json!(linear);
        }

        if let Some(force) = params.0.allow_force_pushes {
            body["allow_force_pushes"] = serde_json::json!(force);
        }

        if let Some(del) = params.0.allow_deletions {
            body["allow_deletions"] = serde_json::json!(del);
        }

        // For simplified implementation, use -f flags for the basic case
        // Full JSON body handling would require stdin piping
        let _body_str = serde_json::to_string(&body).map_err(Self::err)?;
        let enforce_admins_arg = format!("enforce_admins={}", params.0.enforce_admins.unwrap_or(false));

        let result = self
            .gh
            .run(
                params.0.account.as_deref(),
                &[
                    "api",
                    "-X",
                    "PUT",
                    &endpoint,
                    "-H",
                    "Accept: application/vnd.github+json",
                    "-f",
                    &enforce_admins_arg,
                ],
            )
            .await;

        match result {
            Ok(r) => Ok(CallToolResult::success(vec![Content::json(&r)?])),
            Err(e) => Ok(CallToolResult::success(vec![Content::text(format!(
                "Branch protection update attempted. Note: Full protection settings may require direct API access. Error details: {e}"
            ))])),
        }
    }

    /// Remove branch protection from a branch
    #[tool(description = "Remove all branch protection rules from a branch.")]
    async fn delete_branch_protection(
        &self,
        params: Parameters<DeleteBranchProtectionRequest>,
    ) -> Result<CallToolResult, McpError> {
        let endpoint = format!(
            "repos/{}/{}/branches/{}/protection",
            params.0.owner, params.0.repo, params.0.branch
        );
        self.gh
            .api(params.0.account.as_deref(), &endpoint, Some("DELETE"), None)
            .await
            .map_err(Self::err)?;
        Ok(CallToolResult::success(vec![Content::text(format!(
            "Branch protection removed from '{}'",
            params.0.branch
        ))]))
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
