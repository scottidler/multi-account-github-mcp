# multi-account-github-mcp implementation plan

A Rust MCP (Model Context Protocol) server that provides GitHub tools with multi-account support, wrapping the `gh` CLI for implementation.

## problem statement

Using multiple GitHub MCP servers (e.g., `github-home`, `github-work`) for different accounts causes:
- **~60k tokens** of duplicated tool definitions in context
- Nearly **30% of context window** wasted on redundant schemas

## solution

A single MCP server that:
1. Loads **one set of tool definitions** (38 tools)
2. Accepts an `account` parameter to select which GitHub identity to use
3. Wraps `gh` CLI calls with the appropriate `GH_TOKEN` for each account
4. Reduces context usage by **~67%** compared to multiple MCP servers

## architecture

```
┌─────────────────────────────────────────────────────────┐
│              multi-account-github-mcp                   │
├─────────────────────────────────────────────────────────┤
│  Config: multi-account-github-mcp.yml                   │
│          Maps account names to token file paths         │
├─────────────────────────────────────────────────────────┤
│  Tools: Each tool has optional `account` parameter      │
│         Defaults to configured default account          │
├─────────────────────────────────────────────────────────┤
│  Implementation: Spawns `gh` CLI with GH_TOKEN env var  │
│                  Parses JSON output, returns to Claude  │
└─────────────────────────────────────────────────────────┘
```

## configuration

### config file

The project config file (`multi-account-github-mcp.yml`) maps friendly account names to token file paths:

```yaml
# multi-account-github-mcp.yml
default_account: home

accounts:
  home:
    token_path: ~/.config/github/tokens/scottidler
  work:
    token_path: ~/.config/github/tokens/escote-tatari
```

Config file locations (in order of precedence):
1. `--config <path>` CLI argument
2. `~/.config/multi-account-github-mcp/multi-account-github-mcp.yml`
3. `./multi-account-github-mcp.yml`

### token files

Token files are plain text containing a GitHub PAT:

```
~/.config/github/tokens/
├── scottidler           # Personal account token
├── escote-tatari        # Work account token (tatari-tv org)
```

The `work` account uses `escote-tatari` because `tatari-tv` is the org name that appears in repo slugs and URLs.

## tool inventory (38 tools)

### account (1 tool)
| Tool | gh Command | Description |
|------|-----------|-------------|
| `get_me` | `gh api user` | Get authenticated user info |

### repositories (4 tools)
| Tool | gh Command | Description |
|------|-----------|-------------|
| `create_repo` | `gh repo create` | Create a new repository |
| `list_repos` | `gh repo list` | List repositories |
| `get_repo` | `gh repo view` | Get repository details |
| `archive_repo` | `gh api -X PATCH` | Archive a repository |

### branches (3 tools)
| Tool | gh Command | Description |
|------|-----------|-------------|
| `list_branches` | `gh api repos/{o}/{r}/branches` | List branches |
| `create_branch` | `gh api -X POST repos/{o}/{r}/git/refs` | Create a branch |
| `delete_branch` | `gh api -X DELETE repos/{o}/{r}/git/refs/heads/{b}` | Delete a branch |

### branch protection (3 tools)
| Tool | gh Command | Description |
|------|-----------|-------------|
| `get_branch_protection` | `gh api repos/{o}/{r}/branches/{b}/protection` | Get protection rules |
| `set_branch_protection` | `gh api -X PUT repos/{o}/{r}/branches/{b}/protection` | Set protection rules |
| `delete_branch_protection` | `gh api -X DELETE repos/{o}/{r}/branches/{b}/protection` | Remove protection |

### pull requests (10 tools)
| Tool | gh Command | Description |
|------|-----------|-------------|
| `get_pr` | `gh pr view` | Get PR details |
| `get_pr_diff` | `gh pr diff` | Get PR diff |
| `get_pr_files` | `gh pr view --json files` | List files changed in PR |
| `list_prs` | `gh pr list` | List pull requests |
| `search_prs` | `gh search prs` | Search pull requests |
| `create_pr` | `gh pr create` | Create a pull request |
| `edit_pr` | `gh pr edit` | Edit a pull request |
| `merge_pr` | `gh pr merge` | Merge a pull request |
| `close_pr` | `gh pr close` | Close a pull request |
| `comment_pr` | `gh pr comment` | Add comment to PR |

### code and content (3 tools)
| Tool | gh Command | Description |
|------|-----------|-------------|
| `get_file` | `gh api repos/{o}/{r}/contents/{path}` | Get file contents |
| `search_code` | `gh search code` | Search code |
| `list_commits` | `gh api repos/{o}/{r}/commits` | List commits |

### releases (4 tools)
| Tool | gh Command | Description |
|------|-----------|-------------|
| `list_releases` | `gh release list` | List releases |
| `get_release` | `gh release view {tag}` | Get release details |
| `create_release` | `gh release create` | Create a release |
| `delete_release` | `gh release delete {tag}` | Delete a release |

### release assets (2 tools)
| Tool | gh Command | Description |
|------|-----------|-------------|
| `list_release_assets` | `gh api repos/{o}/{r}/releases/{id}/assets` | List release assets |
| `download_release_asset` | `gh release download {tag}` | Download release assets |

### workflow artifacts (3 tools)
| Tool | gh Command | Description |
|------|-----------|-------------|
| `list_workflow_runs` | `gh run list` | List workflow runs |
| `list_run_artifacts` | `gh api repos/{o}/{r}/actions/runs/{id}/artifacts` | List run artifacts |
| `download_run_artifact` | `gh run download {run_id}` | Download artifacts |

### tags (3 tools)
| Tool | gh Command | Description |
|------|-----------|-------------|
| `list_tags` | `gh api repos/{o}/{r}/tags` | List tags |
| `create_tag` | `gh api -X POST repos/{o}/{r}/git/refs` | Create a tag |
| `delete_tag` | `gh api -X DELETE repos/{o}/{r}/git/refs/tags/{t}` | Delete a tag |

### teams and collaborators (5 tools)
| Tool | gh Command | Description |
|------|-----------|-------------|
| `list_collaborators` | `gh api repos/{o}/{r}/collaborators` | List collaborators |
| `add_collaborator` | `gh api -X PUT repos/{o}/{r}/collaborators/{u}` | Add collaborator |
| `remove_collaborator` | `gh api -X DELETE repos/{o}/{r}/collaborators/{u}` | Remove collaborator |
| `list_teams` | `gh api orgs/{org}/teams` | List org teams |
| `get_team_members` | `gh api orgs/{org}/teams/{team}/members` | Get team members |

---

## implementation phases

### phase 1: core infrastructure

**goal**: Establish the foundation - config loading, gh client wrapper, and MCP server skeleton.

#### project structure
```
src/
├── main.rs              # CLI entrypoint, MCP server startup
├── lib.rs               # Library root, re-exports
├── cli.rs               # CLI argument parsing
├── config.rs            # Account/token configuration
├── gh.rs                # gh CLI wrapper
├── error.rs             # Error types
├── mcp/
│   ├── mod.rs           # MCP module root
│   ├── server.rs        # MCP server implementation
│   └── types.rs         # MCP protocol types
└── tools/
    ├── mod.rs           # Tool registry and trait
    └── account.rs       # get_me tool
```

#### deliverables
- [ ] Config loading from yml file with account name to token path mapping
- [ ] Token file reading with path expansion (~/)
- [ ] `GhClient` struct that spawns `gh` with correct `GH_TOKEN`
- [ ] MCP server skeleton using `rmcp` crate
- [ ] Single working tool: `get_me`
- [ ] Comprehensive unit tests for config and gh client
- [ ] CLI with `--help` showing `REQUIRED TOOLS: gh`

#### tests
- Config parsing with valid/invalid yml
- Token file reading and path expansion
- GhClient token selection logic
- GhClient command execution and JSON parsing
- MCP server initialization

---

### phase 2: repository and branch tools

**goal**: Implement repository management and branch operations.

#### tools (10 total)
```
repositories (4):
- create_repo
- list_repos
- get_repo
- archive_repo

branches (3):
- list_branches
- create_branch
- delete_branch

branch protection (3):
- get_branch_protection
- set_branch_protection
- delete_branch_protection
```

#### deliverables
- [ ] All 10 tools implemented and registered
- [ ] Input validation for owner/repo parameters
- [ ] Proper error handling and messages
- [ ] Integration tests using a test repository
- [ ] Unit tests for parameter building

---

### phase 3: pull request tools

**goal**: Implement PR lifecycle management.

#### tools (10 total)
```
- get_pr
- get_pr_diff
- get_pr_files
- list_prs
- search_prs
- create_pr
- edit_pr
- merge_pr
- close_pr
- comment_pr
```

#### deliverables
- [ ] All 10 PR tools implemented
- [ ] Support for PR number and URL inputs
- [ ] Diff output formatting
- [ ] Search query building
- [ ] Unit and integration tests

---

### phase 4: releases, tags, and artifacts

**goal**: Implement release management and artifact access.

#### tools (12 total)
```
code and content (3):
- get_file
- search_code
- list_commits

releases (4):
- list_releases
- get_release
- create_release
- delete_release

release assets (2):
- list_release_assets
- download_release_asset

workflow artifacts (3):
- list_workflow_runs
- list_run_artifacts
- download_run_artifact

tags (3):
- list_tags
- create_tag
- delete_tag
```

#### deliverables
- [ ] All 15 tools implemented
- [ ] File content decoding (base64)
- [ ] Download path handling
- [ ] Tag/release version validation
- [ ] Unit and integration tests

---

### phase 5: teams and collaborators

**goal**: Implement organization and collaboration tools.

#### tools (5 total)
```
- list_collaborators
- add_collaborator
- remove_collaborator
- list_teams
- get_team_members
```

#### deliverables
- [ ] All 5 tools implemented
- [ ] Permission level handling for collaborators
- [ ] Org membership validation
- [ ] Unit and integration tests
- [ ] Full end-to-end testing with both accounts

---

## testing strategy

### unit tests
- Config parsing and token resolution
- GhClient command building
- Tool parameter validation
- JSON response parsing

### integration tests
- Actual `gh` CLI execution (requires auth)
- MCP protocol compliance
- Multi-account switching

### end-to-end tests
- Full tool invocation via MCP
- Test with `home` (scottidler) account
- Test with `work` (escote-tatari / tatari-tv) account

### test accounts
| Alias | Token File | Org/User | Use Case |
|-------|------------|----------|----------|
| `home` | `scottidler` | scottidler | Personal repos |
| `work` | `escote-tatari` | tatari-tv | Work/org repos |

---

## claude code integration

### mcp server configuration

Add to `~/.claude/settings.json`:

```json
{
  "mcpServers": {
    "github": {
      "command": "multi-account-github-mcp",
      "args": ["serve"],
      "env": {}
    }
  }
}
```

### usage examples

```
# Using default account (home)
get_me()

# Explicitly specifying account
list_repos(account: "work", owner: "tatari-tv")

# Creating a PR with work account
create_pr(account: "work", owner: "tatari-tv", repo: "some-repo", ...)
```

---

## dependencies

### required tools
- `gh` CLI (>= 2.0) - GitHub CLI for API access

### rust version
- Rust 1.75+ (edition 2024)

### crates
```toml
[dependencies]
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
serde_yaml = "0.9"
clap = { version = "4", features = ["derive"] }
eyre = "0.6"
rmcp = "0.1"
dirs = "6"
tracing = "0.1"
tracing-subscriber = "0.3"
shellexpand = "3"

[dev-dependencies]
tempfile = "3"
assert_cmd = "2"
predicates = "3"
```

---

## cli interface

```
multi-account-github-mcp - GitHub MCP server with multi-account support

USAGE:
    multi-account-github-mcp <COMMAND>

COMMANDS:
    serve       Start the MCP server (stdio transport)
    accounts    List configured accounts
    test        Test connection for an account
    help        Print help information

OPTIONS:
    -c, --config <FILE>    Config file path
    -v, --verbose          Enable verbose logging
    -h, --help             Print help
    -V, --version          Print version

REQUIRED TOOLS:
    gh    GitHub CLI (https://cli.github.com)

CONFIGURATION:
    Default config: ~/.config/multi-account-github-mcp/multi-account-github-mcp.yml
    Token files:    Specified in config (e.g., ~/.config/github/tokens/<name>)
```

---

## context savings analysis

| Configuration | Tools | Estimated Tokens |
|---------------|-------|------------------|
| github-home MCP | 41 | ~29k |
| github-work MCP | 41 | ~29k |
| github MCP | 41 | ~29k |
| **Total (current)** | **123** | **~87k** |
| | | |
| multi-account-github-mcp | 38 | ~27k |
| **Savings** | **85 tools** | **~60k (69%)** |
