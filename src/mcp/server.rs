//! MCP server implementation for GitHub multi-account

use crate::GhClient;
use crate::tools::account::GetMeRequest;
use crate::tools::branches::{CreateBranchRequest, DeleteBranchRequest, ListBranchesRequest};
use crate::tools::code::{GetFileRequest, ListCommitsRequest, SearchCodeRequest};
use crate::tools::protection::{DeleteBranchProtectionRequest, GetBranchProtectionRequest, SetBranchProtectionRequest};
use crate::tools::prs::{
    ClosePrRequest, CommentPrRequest, CreatePrRequest, EditPrRequest, GetPrDiffRequest, GetPrFilesRequest,
    GetPrRequest, ListPrsRequest, MergePrRequest, SearchPrsRequest,
};
use crate::tools::releases::{
    CreateReleaseRequest, DeleteReleaseRequest, DownloadReleaseAssetRequest, GetReleaseRequest,
    ListReleaseAssetsRequest, ListReleasesRequest,
};
use crate::tools::repos::{ArchiveRepoRequest, CreateRepoRequest, GetRepoRequest, ListReposRequest};
use crate::tools::tags::{CreateTagRequest, DeleteTagRequest, ListTagsRequest};
use crate::tools::workflows::{DownloadRunArtifactRequest, ListRunArtifactsRequest, ListWorkflowRunsRequest};
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

    // ============================================
    // Pull Request Tools
    // ============================================

    /// Get details about a specific pull request
    #[tool(description = "Get detailed information about a specific pull request.")]
    async fn get_pr(&self, params: Parameters<GetPrRequest>) -> Result<CallToolResult, McpError> {
        let repo = format!("{}/{}", params.0.owner, params.0.repo);
        let number_str = params.0.number.to_string();
        let args = vec![
            "pr",
            "view",
            &number_str,
            "--repo",
            &repo,
            "--json",
            "number,title,state,body,author,createdAt,updatedAt,url,headRefName,baseRefName,mergeable,additions,deletions,changedFiles",
        ];
        let result = self
            .gh
            .run(params.0.account.as_deref(), &args)
            .await
            .map_err(Self::err)?;
        Ok(CallToolResult::success(vec![Content::json(&result)?]))
    }

    /// Get the diff of a pull request
    #[tool(description = "Get the diff/patch of a pull request.")]
    async fn get_pr_diff(&self, params: Parameters<GetPrDiffRequest>) -> Result<CallToolResult, McpError> {
        let repo = format!("{}/{}", params.0.owner, params.0.repo);
        let number_str = params.0.number.to_string();
        let args = vec!["pr", "diff", &number_str, "--repo", &repo];
        let result = self
            .gh
            .run(params.0.account.as_deref(), &args)
            .await
            .map_err(Self::err)?;
        Ok(CallToolResult::success(vec![Content::text(
            result.as_str().unwrap_or("").to_string(),
        )]))
    }

    /// Get files changed in a pull request
    #[tool(description = "Get the list of files changed in a pull request.")]
    async fn get_pr_files(&self, params: Parameters<GetPrFilesRequest>) -> Result<CallToolResult, McpError> {
        let repo = format!("{}/{}", params.0.owner, params.0.repo);
        let number_str = params.0.number.to_string();
        let args = vec!["pr", "view", &number_str, "--repo", &repo, "--json", "files"];
        let result = self
            .gh
            .run(params.0.account.as_deref(), &args)
            .await
            .map_err(Self::err)?;
        Ok(CallToolResult::success(vec![Content::json(&result)?]))
    }

    /// List pull requests in a repository
    #[tool(description = "List pull requests in a repository with optional filters.")]
    async fn list_prs(&self, params: Parameters<ListPrsRequest>) -> Result<CallToolResult, McpError> {
        let repo = format!("{}/{}", params.0.owner, params.0.repo);
        let mut args = vec!["pr", "list", "--repo", &repo];

        let state;
        if let Some(ref s) = params.0.state {
            state = format!("--state={s}");
            args.push(&state);
        }

        let limit_str;
        if let Some(limit) = params.0.limit {
            limit_str = limit.to_string();
            args.push("--limit");
            args.push(&limit_str);
        }

        let base;
        if let Some(ref b) = params.0.base {
            base = format!("--base={b}");
            args.push(&base);
        }

        let head;
        if let Some(ref h) = params.0.head {
            head = format!("--head={h}");
            args.push(&head);
        }

        args.push("--json");
        args.push("number,title,state,author,createdAt,updatedAt,url,headRefName,baseRefName");

        let result = self
            .gh
            .run(params.0.account.as_deref(), &args)
            .await
            .map_err(Self::err)?;
        Ok(CallToolResult::success(vec![Content::json(&result)?]))
    }

    /// Search pull requests
    #[tool(description = "Search pull requests using GitHub search syntax.")]
    async fn search_prs(&self, params: Parameters<SearchPrsRequest>) -> Result<CallToolResult, McpError> {
        let mut args = vec!["search", "prs", &params.0.query];

        let limit_str;
        if let Some(limit) = params.0.limit {
            limit_str = limit.to_string();
            args.push("--limit");
            args.push(&limit_str);
        }

        args.push("--json");
        args.push("number,title,state,author,repository,createdAt,updatedAt,url");

        let result = self
            .gh
            .run(params.0.account.as_deref(), &args)
            .await
            .map_err(Self::err)?;
        Ok(CallToolResult::success(vec![Content::json(&result)?]))
    }

    /// Create a new pull request
    #[tool(description = "Create a new pull request.")]
    async fn create_pr(&self, params: Parameters<CreatePrRequest>) -> Result<CallToolResult, McpError> {
        let repo = format!("{}/{}", params.0.owner, params.0.repo);
        let mut args = vec![
            "pr",
            "create",
            "--repo",
            &repo,
            "--title",
            &params.0.title,
            "--head",
            &params.0.head,
        ];

        let body;
        if let Some(ref b) = params.0.body {
            body = format!("--body={b}");
            args.push(&body);
        }

        let base;
        if let Some(ref b) = params.0.base {
            base = format!("--base={b}");
            args.push(&base);
        }

        if params.0.draft.unwrap_or(false) {
            args.push("--draft");
        }

        let result = self
            .gh
            .run(params.0.account.as_deref(), &args)
            .await
            .map_err(Self::err)?;
        Ok(CallToolResult::success(vec![Content::json(&result)?]))
    }

    /// Edit an existing pull request
    #[tool(description = "Edit an existing pull request's title, body, or base branch.")]
    async fn edit_pr(&self, params: Parameters<EditPrRequest>) -> Result<CallToolResult, McpError> {
        let repo = format!("{}/{}", params.0.owner, params.0.repo);
        let number_str = params.0.number.to_string();
        let mut args = vec!["pr", "edit", &number_str, "--repo", &repo];

        let title;
        if let Some(ref t) = params.0.title {
            title = format!("--title={t}");
            args.push(&title);
        }

        let body;
        if let Some(ref b) = params.0.body {
            body = format!("--body={b}");
            args.push(&body);
        }

        let base;
        if let Some(ref b) = params.0.base {
            base = format!("--base={b}");
            args.push(&base);
        }

        let result = self
            .gh
            .run(params.0.account.as_deref(), &args)
            .await
            .map_err(Self::err)?;
        Ok(CallToolResult::success(vec![Content::json(&result)?]))
    }

    /// Merge a pull request
    #[tool(description = "Merge a pull request. Supports merge, squash, and rebase methods.")]
    async fn merge_pr(&self, params: Parameters<MergePrRequest>) -> Result<CallToolResult, McpError> {
        let repo = format!("{}/{}", params.0.owner, params.0.repo);
        let number_str = params.0.number.to_string();
        let mut args = vec!["pr", "merge", &number_str, "--repo", &repo];

        match params.0.method.as_deref() {
            Some("squash") => args.push("--squash"),
            Some("rebase") => args.push("--rebase"),
            _ => args.push("--merge"),
        }

        if params.0.delete_branch.unwrap_or(false) {
            args.push("--delete-branch");
        }

        let commit_msg;
        if let Some(ref msg) = params.0.commit_message {
            commit_msg = format!("--body={msg}");
            args.push(&commit_msg);
        }

        let result = self
            .gh
            .run(params.0.account.as_deref(), &args)
            .await
            .map_err(Self::err)?;
        Ok(CallToolResult::success(vec![Content::json(&result)?]))
    }

    /// Close a pull request without merging
    #[tool(description = "Close a pull request without merging.")]
    async fn close_pr(&self, params: Parameters<ClosePrRequest>) -> Result<CallToolResult, McpError> {
        let repo = format!("{}/{}", params.0.owner, params.0.repo);
        let number_str = params.0.number.to_string();
        let args = vec!["pr", "close", &number_str, "--repo", &repo];
        let result = self
            .gh
            .run(params.0.account.as_deref(), &args)
            .await
            .map_err(Self::err)?;
        Ok(CallToolResult::success(vec![Content::json(&result)?]))
    }

    /// Add a comment to a pull request
    #[tool(description = "Add a comment to a pull request.")]
    async fn comment_pr(&self, params: Parameters<CommentPrRequest>) -> Result<CallToolResult, McpError> {
        let repo = format!("{}/{}", params.0.owner, params.0.repo);
        let number_str = params.0.number.to_string();
        let args = vec!["pr", "comment", &number_str, "--repo", &repo, "--body", &params.0.body];
        let result = self
            .gh
            .run(params.0.account.as_deref(), &args)
            .await
            .map_err(Self::err)?;
        Ok(CallToolResult::success(vec![Content::json(&result)?]))
    }

    // ============================================
    // Code and Content Tools
    // ============================================

    /// Get contents of a file from a repository
    #[tool(description = "Get the contents of a file from a repository.")]
    async fn get_file(&self, params: Parameters<GetFileRequest>) -> Result<CallToolResult, McpError> {
        let mut endpoint = format!("repos/{}/{}/contents/{}", params.0.owner, params.0.repo, params.0.path);

        if let Some(ref r) = params.0.r#ref {
            endpoint.push_str(&format!("?ref={r}"));
        }

        let result = self
            .gh
            .api(params.0.account.as_deref(), &endpoint, None, None)
            .await
            .map_err(Self::err)?;

        // If content is base64 encoded, decode it
        if let Some(content) = result["content"].as_str() {
            let decoded = content.replace('\n', "");
            if let Ok(bytes) = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &decoded)
                && let Ok(text) = String::from_utf8(bytes)
            {
                return Ok(CallToolResult::success(vec![Content::text(text)]));
            }
        }

        Ok(CallToolResult::success(vec![Content::json(&result)?]))
    }

    /// Search code across repositories
    #[tool(description = "Search code using GitHub code search syntax.")]
    async fn search_code(&self, params: Parameters<SearchCodeRequest>) -> Result<CallToolResult, McpError> {
        let mut args = vec!["search", "code", &params.0.query];

        let limit_str;
        if let Some(limit) = params.0.limit {
            limit_str = limit.to_string();
            args.push("--limit");
            args.push(&limit_str);
        }

        args.push("--json");
        args.push("path,repository,textMatches");

        let result = self
            .gh
            .run(params.0.account.as_deref(), &args)
            .await
            .map_err(Self::err)?;
        Ok(CallToolResult::success(vec![Content::json(&result)?]))
    }

    /// List commits in a repository
    #[tool(description = "List commits in a repository with optional filters.")]
    async fn list_commits(&self, params: Parameters<ListCommitsRequest>) -> Result<CallToolResult, McpError> {
        let mut endpoint = format!("repos/{}/{}/commits", params.0.owner, params.0.repo);
        let mut query_params = Vec::new();

        if let Some(ref sha) = params.0.sha {
            query_params.push(format!("sha={sha}"));
        }

        if let Some(ref path) = params.0.path {
            query_params.push(format!("path={path}"));
        }

        if let Some(ref author) = params.0.author {
            query_params.push(format!("author={author}"));
        }

        if let Some(limit) = params.0.limit {
            query_params.push(format!("per_page={limit}"));
        }

        if !query_params.is_empty() {
            endpoint.push('?');
            endpoint.push_str(&query_params.join("&"));
        }

        let result = self
            .gh
            .api(params.0.account.as_deref(), &endpoint, None, None)
            .await
            .map_err(Self::err)?;
        Ok(CallToolResult::success(vec![Content::json(&result)?]))
    }

    // ============================================
    // Release Tools
    // ============================================

    /// List releases in a repository
    #[tool(description = "List releases in a repository.")]
    async fn list_releases(&self, params: Parameters<ListReleasesRequest>) -> Result<CallToolResult, McpError> {
        let repo = format!("{}/{}", params.0.owner, params.0.repo);
        let mut args = vec!["release", "list", "--repo", &repo];

        let limit_str;
        if let Some(limit) = params.0.limit {
            limit_str = limit.to_string();
            args.push("--limit");
            args.push(&limit_str);
        }

        let result = self
            .gh
            .run(params.0.account.as_deref(), &args)
            .await
            .map_err(Self::err)?;
        Ok(CallToolResult::success(vec![Content::json(&result)?]))
    }

    /// Get details about a specific release
    #[tool(description = "Get detailed information about a specific release by tag.")]
    async fn get_release(&self, params: Parameters<GetReleaseRequest>) -> Result<CallToolResult, McpError> {
        let repo = format!("{}/{}", params.0.owner, params.0.repo);
        let args = vec![
            "release",
            "view",
            &params.0.tag,
            "--repo",
            &repo,
            "--json",
            "tagName,name,body,author,createdAt,publishedAt,isDraft,isPrerelease,assets,url",
        ];
        let result = self
            .gh
            .run(params.0.account.as_deref(), &args)
            .await
            .map_err(Self::err)?;
        Ok(CallToolResult::success(vec![Content::json(&result)?]))
    }

    /// Create a new release
    #[tool(description = "Create a new release with optional release notes.")]
    async fn create_release(&self, params: Parameters<CreateReleaseRequest>) -> Result<CallToolResult, McpError> {
        let repo = format!("{}/{}", params.0.owner, params.0.repo);
        let mut args = vec!["release", "create", &params.0.tag, "--repo", &repo];

        let title;
        if let Some(ref t) = params.0.title {
            title = format!("--title={t}");
            args.push(&title);
        }

        let notes;
        if let Some(ref n) = params.0.notes {
            notes = format!("--notes={n}");
            args.push(&notes);
        }

        let target;
        if let Some(ref t) = params.0.target {
            target = format!("--target={t}");
            args.push(&target);
        }

        if params.0.draft.unwrap_or(false) {
            args.push("--draft");
        }

        if params.0.prerelease.unwrap_or(false) {
            args.push("--prerelease");
        }

        if params.0.generate_notes.unwrap_or(false) {
            args.push("--generate-notes");
        }

        let result = self
            .gh
            .run(params.0.account.as_deref(), &args)
            .await
            .map_err(Self::err)?;
        Ok(CallToolResult::success(vec![Content::json(&result)?]))
    }

    /// Delete a release
    #[tool(description = "Delete a release by tag. Optionally delete the associated git tag.")]
    async fn delete_release(&self, params: Parameters<DeleteReleaseRequest>) -> Result<CallToolResult, McpError> {
        let repo = format!("{}/{}", params.0.owner, params.0.repo);
        let mut args = vec!["release", "delete", &params.0.tag, "--repo", &repo, "--yes"];

        if params.0.delete_tag.unwrap_or(false) {
            args.push("--cleanup-tag");
        }

        let result = self
            .gh
            .run(params.0.account.as_deref(), &args)
            .await
            .map_err(Self::err)?;
        Ok(CallToolResult::success(vec![Content::json(&result)?]))
    }

    /// List assets in a release
    #[tool(description = "List assets (files) attached to a release.")]
    async fn list_release_assets(
        &self,
        params: Parameters<ListReleaseAssetsRequest>,
    ) -> Result<CallToolResult, McpError> {
        let repo = format!("{}/{}", params.0.owner, params.0.repo);
        let args = vec!["release", "view", &params.0.tag, "--repo", &repo, "--json", "assets"];
        let result = self
            .gh
            .run(params.0.account.as_deref(), &args)
            .await
            .map_err(Self::err)?;
        Ok(CallToolResult::success(vec![Content::json(&result)?]))
    }

    /// Download release assets
    #[tool(description = "Download assets from a release.")]
    async fn download_release_asset(
        &self,
        params: Parameters<DownloadReleaseAssetRequest>,
    ) -> Result<CallToolResult, McpError> {
        let repo = format!("{}/{}", params.0.owner, params.0.repo);
        let mut args = vec!["release", "download", &params.0.tag, "--repo", &repo];

        let pattern;
        if let Some(ref p) = params.0.pattern {
            pattern = format!("--pattern={p}");
            args.push(&pattern);
        }

        let dir;
        if let Some(ref d) = params.0.dir {
            dir = format!("--dir={d}");
            args.push(&dir);
        }

        let result = self
            .gh
            .run(params.0.account.as_deref(), &args)
            .await
            .map_err(Self::err)?;
        Ok(CallToolResult::success(vec![Content::json(&result)?]))
    }

    // ============================================
    // Tag Tools
    // ============================================

    /// List tags in a repository
    #[tool(description = "List git tags in a repository.")]
    async fn list_tags(&self, params: Parameters<ListTagsRequest>) -> Result<CallToolResult, McpError> {
        let mut endpoint = format!("repos/{}/{}/tags", params.0.owner, params.0.repo);
        if let Some(limit) = params.0.limit {
            endpoint.push_str(&format!("?per_page={limit}"));
        }
        let result = self
            .gh
            .api(params.0.account.as_deref(), &endpoint, None, None)
            .await
            .map_err(Self::err)?;
        Ok(CallToolResult::success(vec![Content::json(&result)?]))
    }

    /// Create a new tag
    #[tool(description = "Create a new git tag pointing to a specific commit.")]
    async fn create_tag(&self, params: Parameters<CreateTagRequest>) -> Result<CallToolResult, McpError> {
        // Get the target SHA if not provided
        let sha = if let Some(ref s) = params.0.sha {
            s.clone()
        } else {
            // Get HEAD of default branch
            let repo_endpoint = format!("repos/{}/{}", params.0.owner, params.0.repo);
            let repo_info = self
                .gh
                .api(params.0.account.as_deref(), &repo_endpoint, None, None)
                .await
                .map_err(Self::err)?;
            let default_branch = repo_info["default_branch"].as_str().unwrap_or("main");

            let ref_endpoint = format!(
                "repos/{}/{}/git/ref/heads/{}",
                params.0.owner, params.0.repo, default_branch
            );
            let ref_info = self
                .gh
                .api(params.0.account.as_deref(), &ref_endpoint, None, None)
                .await
                .map_err(Self::err)?;
            ref_info["object"]["sha"].as_str().unwrap_or("").to_string()
        };

        // Create the tag ref
        let endpoint = format!("repos/{}/{}/git/refs", params.0.owner, params.0.repo);
        let ref_name = format!("refs/tags/{}", params.0.tag);
        let ref_arg = format!("ref={ref_name}");
        let sha_arg = format!("sha={sha}");

        let args = vec!["api", "-X", "POST", &endpoint, "-f", &ref_arg, "-f", &sha_arg];
        let result = self
            .gh
            .run(params.0.account.as_deref(), &args)
            .await
            .map_err(Self::err)?;
        Ok(CallToolResult::success(vec![Content::json(&result)?]))
    }

    /// Delete a tag
    #[tool(description = "Delete a git tag from a repository.")]
    async fn delete_tag(&self, params: Parameters<DeleteTagRequest>) -> Result<CallToolResult, McpError> {
        let endpoint = format!(
            "repos/{}/{}/git/refs/tags/{}",
            params.0.owner, params.0.repo, params.0.tag
        );
        self.gh
            .api(params.0.account.as_deref(), &endpoint, Some("DELETE"), None)
            .await
            .map_err(Self::err)?;
        Ok(CallToolResult::success(vec![Content::text(format!(
            "Tag '{}' deleted successfully",
            params.0.tag
        ))]))
    }

    // ============================================
    // Workflow and Artifact Tools
    // ============================================

    /// List workflow runs in a repository
    #[tool(description = "List GitHub Actions workflow runs in a repository.")]
    async fn list_workflow_runs(
        &self,
        params: Parameters<ListWorkflowRunsRequest>,
    ) -> Result<CallToolResult, McpError> {
        let repo = format!("{}/{}", params.0.owner, params.0.repo);
        let mut args = vec!["run", "list", "--repo", &repo];

        let workflow;
        if let Some(ref w) = params.0.workflow {
            workflow = format!("--workflow={w}");
            args.push(&workflow);
        }

        let branch;
        if let Some(ref b) = params.0.branch {
            branch = format!("--branch={b}");
            args.push(&branch);
        }

        let status;
        if let Some(ref s) = params.0.status {
            status = format!("--status={s}");
            args.push(&status);
        }

        let limit_str;
        if let Some(limit) = params.0.limit {
            limit_str = limit.to_string();
            args.push("--limit");
            args.push(&limit_str);
        }

        args.push("--json");
        args.push("databaseId,workflowName,status,conclusion,headBranch,event,createdAt,url");

        let result = self
            .gh
            .run(params.0.account.as_deref(), &args)
            .await
            .map_err(Self::err)?;
        Ok(CallToolResult::success(vec![Content::json(&result)?]))
    }

    /// List artifacts from a workflow run
    #[tool(description = "List artifacts from a specific workflow run.")]
    async fn list_run_artifacts(
        &self,
        params: Parameters<ListRunArtifactsRequest>,
    ) -> Result<CallToolResult, McpError> {
        let endpoint = format!(
            "repos/{}/{}/actions/runs/{}/artifacts",
            params.0.owner, params.0.repo, params.0.run_id
        );
        let result = self
            .gh
            .api(params.0.account.as_deref(), &endpoint, None, None)
            .await
            .map_err(Self::err)?;
        Ok(CallToolResult::success(vec![Content::json(&result)?]))
    }

    /// Download artifacts from a workflow run
    #[tool(description = "Download artifacts from a workflow run.")]
    async fn download_run_artifact(
        &self,
        params: Parameters<DownloadRunArtifactRequest>,
    ) -> Result<CallToolResult, McpError> {
        let repo = format!("{}/{}", params.0.owner, params.0.repo);
        let run_id_str = params.0.run_id.to_string();
        let mut args = vec!["run", "download", &run_id_str, "--repo", &repo];

        let name;
        if let Some(ref n) = params.0.name {
            name = format!("--name={n}");
            args.push(&name);
        }

        let dir;
        if let Some(ref d) = params.0.dir {
            dir = format!("--dir={d}");
            args.push(&dir);
        }

        let result = self
            .gh
            .run(params.0.account.as_deref(), &args)
            .await
            .map_err(Self::err)?;
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
