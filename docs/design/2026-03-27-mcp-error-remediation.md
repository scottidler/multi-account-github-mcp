# Design Document: MCP Error Remediation

**Author:** Scott Idler + Claude
**Date:** 2026-03-27
**Status:** Implemented
**Review Passes Completed:** 5/5

## Summary

Fix four categories of errors observed across Claude Code session logs when using the multi-account-github MCP server. The errors range from code bugs (JSON parse failures, type coercion) to operational gaps (missing scope diagnostics, rate limiting). Combined, these produce ~100+ errors across recent sessions.

## Problem Statement

### Background

The multi-account-github MCP server wraps the `gh` CLI to provide GitHub operations to Claude Code via the MCP protocol. It handles ~40 tools across repos, PRs, branches, releases, tags, workflows, and teams. Each tool invocation shells out to `gh` and parses the result.

### Problem

Analysis of session logs reveals four error patterns accounting for the vast majority of MCP failures:

| # | Error Pattern | Occurrences | Root Cause |
|---|--------------|-------------|------------|
| 1 | `Failed to parse gh output as JSON` | 41 | `gh` subcommands that return plain text (URLs, tables) routed through `run()` which expects JSON |
| 2 | `missing_scope` | 24 | PAT lacks required OAuth scope; error message gives zero diagnostic context |
| 3 | `HTTP 403: API rate limit exceeded` | 22 | No rate limit awareness, detection, or backoff |
| 4 | `failed to deserialize parameters: invalid type: string "23", expected u64` | 16 | LLM sends `number` as string; serde rejects it |

### Goals

- Eliminate JSON parse failures by correctly handling non-JSON `gh` output
- Make `missing_scope` errors actionable by logging the command, account, and required scopes
- Detect rate limiting and surface remaining quota, reset time, and retry guidance
- Accept string-encoded integers from LLM callers without breaking existing numeric input

### Non-Goals

- Implementing automatic retry/backoff (scope creep; surface info and let callers decide)
- Adding OAuth scope checking at startup (would need a scope-per-tool map, over-engineered)
- Changing the MCP protocol or rmcp framework behavior
- Fixing `missing_scope` itself (that's a PAT configuration issue, not a code fix)

## Proposed Solution

### Overview

Four targeted fixes, each independent of the others:

1. **Non-JSON output handling** - tools that call `gh` subcommands without `--json` support switch from `run()` to `run_raw()`, or add `--json` where supported
2. **Scope error diagnostics** - parse `missing_scope` from `gh` stderr, log the full context (account, command, scopes), and return an actionable error message
3. **Rate limit detection** - parse 403 rate-limit responses, extract headers (limit, remaining, reset), and include them in the error
4. **Flexible integer deserialization** - add a serde helper that accepts both `42` and `"42"` for all `number`/`limit`/`u64` fields

### Phase 1: Non-JSON Output Handling

**Problem:** `GhClient::run()` always calls `serde_json::from_str()` on stdout. Several `gh` subcommands don't return JSON - they return URLs or tab-delimited text. These commands have no `--json` flag.

**Affected tools and their `gh` output:**

| Tool | `gh` Command | Output Type | Fix |
|------|-------------|-------------|-----|
| `create_repo` | `gh repo create` | URL string | `run_raw()` + `Content::text()` |
| `create_pr` | `gh pr create` | URL string | `run_raw()` + `Content::text()` |
| `comment_pr` | `gh pr comment` | URL string | `run_raw()` + `Content::text()` |
| `edit_pr` | `gh pr edit` | URL string | `run_raw()` + `Content::text()` |
| `merge_pr` | `gh pr merge` | text message | `run_raw()` + `Content::text()` |
| `close_pr` | `gh pr close` | text message | `run_raw()` + `Content::text()` |
| `get_pr_diff` | `gh pr diff` | raw diff text | `run_raw()` + `Content::text()` (already returns text but calls `run()` first) |
| `create_release` | `gh release create` | URL/text | `run_raw()` + `Content::text()` |
| `delete_release` | `gh release delete` | text message | `run_raw()` + `Content::text()` |
| `download_release_asset` | `gh release download` | text message | `run_raw()` + `Content::text()` |
| `list_releases` | `gh release list` | tab-delimited | Add `--json` fields |

**Implementation:**

For tools where `gh` has no `--json` flag, switch from `self.gh.run()` to `self.gh.run_raw()` and return `Content::text()` instead of `Content::json()`.

For `list_releases`, add `--json` fields (like `list_repos` and `list_prs` already do):
```rust
args.push("--json");
args.push("tagName,name,createdAt,publishedAt,isDraft,isLatest,isPrerelease");
```

### Phases 2 & 3: Error Detection in `GhClient`

Both scope errors and rate limiting are detected in the same place: the error-handling path inside `GhClient::run()` and `GhClient::run_raw()`. To avoid duplication, extract a shared helper:

```rust
// In src/gh.rs - new private helper
fn classify_gh_error(
    error_msg: &str,
    account: Option<&str>,
    args: &[&str],
) -> Error {
    let account_label = account.unwrap_or("default");
    let command = args.join(" ");

    // Check rate limit first (most specific)
    // Catches both primary ("API rate limit exceeded") and secondary ("secondary rate limit") limits
    if error_msg.contains("rate limit exceeded")
        || error_msg.contains("secondary rate limit")
        || (error_msg.contains("HTTP 403") && error_msg.contains("rate limit"))
    {
        let reset_at = extract_rate_limit_reset(error_msg);
        tracing::warn!(
            account = account_label,
            command = %command,
            reset = %reset_at,
            "GitHub API rate limit exceeded"
        );
        return Error::RateLimit {
            account: account_label.to_string(),
            reset_at,
        };
    }

    // Check scope errors
    if error_msg.contains("missing_scope") || error_msg.contains("insufficient_scope") {
        tracing::warn!(
            account = account_label,
            command = %command,
            "GitHub API scope error - PAT may lack required permissions"
        );
        return Error::GhCli(format!(
            "Missing OAuth scope for account '{}'. Command: gh {}. \
             Check that the PAT has the required scopes. Error: {}",
            account_label, command, error_msg.trim()
        ));
    }

    // Generic error (existing behavior)
    Error::GhCli(error_msg.trim().to_string())
}
```

**Check ordering matters:** rate limit first (most specific), then scope errors, then generic fallback. A rate limit 403 could theoretically contain "scope" text, so rate limit detection must come first.

Both `run()` and `run_raw()` replace their error-handling block to call this helper:

```rust
// Before (in both run() and run_raw()):
if !output.status.success() {
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let error_msg = if stderr.is_empty() { stdout.to_string() } else { stderr.to_string() };
    return Err(Error::GhCli(error_msg.trim().to_string()));
}

// After:
if !output.status.success() {
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let error_msg = if stderr.is_empty() { stdout.to_string() } else { stderr.to_string() };
    return Err(classify_gh_error(&error_msg, account, args));
}
```

**New error variant** in `src/error.rs`:

```rust
#[error("GitHub API rate limit exceeded for account '{account}': resets at {reset_at}")]
RateLimit {
    account: String,
    reset_at: String,
},
```

**Timestamp parser** in `src/gh.rs`:

```rust
fn extract_rate_limit_reset(error_msg: &str) -> String {
    // GitHub error format: "...timestamp 2026-03-10 03:06:38 UTC..."
    if let Some(idx) = error_msg.find("timestamp ") {
        let start = idx + "timestamp ".len();
        if let Some(end) = error_msg[start..].find(" UTC") {
            return format!("{} UTC", &error_msg[start..start + end]);
        }
    }
    "unknown".to_string()
}
```

### Phase 4: Flexible Integer Deserialization

**Problem:** LLMs sometimes send `"23"` (string) instead of `23` (number) for PR numbers, limits, etc. Serde's default `u64` deserializer rejects strings.

**Implementation:**

Add a serde helper module and apply it to all integer fields in request types:

```rust
// In src/serde.rs (new module)
use serde::{Deserialize, Deserializer};

/// Deserialize a u64 from either a number or a string
pub fn flexible_u64<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StringOrU64 {
        U64(u64),
        Str(String),
    }

    match StringOrU64::deserialize(deserializer)? {
        StringOrU64::U64(v) => Ok(v),
        StringOrU64::Str(s) => s.trim().parse::<u64>().map_err(serde::de::Error::custom),
    }
}

/// Same for Option<u64>
pub fn flexible_u64_opt<'de, D>(deserializer: D) -> Result<Option<u64>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StringOrU64 {
        U64(u64),
        Str(String),
    }

    let opt: Option<StringOrU64> = Option::deserialize(deserializer)?;
    match opt {
        None => Ok(None),
        Some(StringOrU64::U64(v)) => Ok(Some(v)),
        Some(StringOrU64::Str(s)) => s
            .trim()
            .parse::<u64>()
            .map(Some)
            .map_err(serde::de::Error::custom),
    }
}
```

Apply to all request struct integer fields:

```rust
#[serde(deserialize_with = "crate::serde::flexible_u64")]
pub number: u64,

#[serde(deserialize_with = "crate::serde::flexible_u64_opt")]
pub limit: Option<u64>,
```

**Affected structs:** `GetPrRequest`, `GetPrDiffRequest`, `GetPrFilesRequest`, `EditPrRequest`, `MergePrRequest`, `ClosePrRequest`, `CommentPrRequest`, `ListPrsRequest`, `SearchPrsRequest`, `ListReposRequest`, `ListReleasesRequest`, `ListWorkflowRunsRequest`, `SearchCodeRequest`, `ListCommitsRequest`.

Also change `limit` fields from `Option<u32>` to `Option<u64>` for consistency - no reason to use different integer widths.

### Data Model

No schema changes. Only changes:
- New `RateLimit` variant on `Error` enum
- New `src/serde.rs` module for flexible deserialization

### Implementation Plan

| Phase | Description | Files Changed |
|-------|------------|---------------|
| 1 | Non-JSON output handling | `src/mcp/server.rs` |
| 2-3 | Error classification (scope + rate limit) | `src/error.rs`, `src/gh.rs` |
| 4 | Flexible integer deser | `src/serde.rs` (new), `src/tools/*.rs`, `src/lib.rs` |

## Alternatives Considered

### Alternative 1: Auto-retry with exponential backoff for rate limits
- **Description:** Automatically retry rate-limited requests after sleeping
- **Pros:** Transparent to callers
- **Cons:** MCP calls would block for potentially minutes; MCP has its own timeout semantics; caller (Claude) can't make informed decisions about whether to wait
- **Why not chosen:** Better to surface rate limit info and let the caller decide. The MCP server shouldn't silently block.

### Alternative 2: Wrapper types instead of serde helpers for flexible integers
- **Description:** Create a `FlexU64` newtype wrapping `u64` with custom Deserialize
- **Pros:** Single-point change per type
- **Cons:** Leaks into all tool handler code (`.0` or `.into()` everywhere), changes the public API of request structs, complicates JsonSchema derivation
- **Why not chosen:** `deserialize_with` attribute is less invasive and keeps fields as plain `u64`

### Alternative 3: Always use `run_raw()` and parse JSON opportunistically
- **Description:** Have `run()` try JSON parse but fall back to raw text
- **Pros:** Single code path, never fails on non-JSON
- **Cons:** Masks real JSON parse errors when a command _should_ return JSON but doesn't (e.g., auth failures returning HTML). Also changes the return type to something less ergonomic.
- **Why not chosen:** Explicit is better. Tools know at compile time whether to expect JSON or text.

### Alternative 4: Check OAuth scopes at startup
- **Description:** Call `gh api user` for each account at startup and log available scopes
- **Pros:** Proactive detection
- **Cons:** Adds startup latency, requires maintaining a scope-per-tool mapping, scopes can change between startup and use
- **Why not chosen:** Over-engineered. Better to diagnose at point of failure with full context.

## Technical Considerations

### Dependencies

No new external dependencies. All changes use existing crates:
- `serde` (already present) for custom deserializers
- `tracing` (already present) for structured logging

### Performance

- No performance impact. Phase 1 actually removes an unnecessary JSON parse attempt.
- Rate limit checking is lazy (only parses error output), not proactive.

### Security

- No security implications. Token handling is unchanged.
- Error messages intentionally exclude token values.

### Testing Strategy

**Phase 1:** Unit tests for each modified tool handler verifying that raw text output is handled correctly. Add tests in `server.rs` tests module.

**Phases 2-3:** Unit tests for `classify_gh_error()`:
- Input containing `missing_scope` returns enriched error with account and command context
- Input containing `API rate limit exceeded` returns `Error::RateLimit` with parsed reset timestamp
- Input with neither returns generic `Error::GhCli` (unchanged behavior)
- Test `extract_rate_limit_reset()` with sample GitHub 403 error messages (with and without timestamp)

**Phase 4:** Serde tests for `flexible_u64`:
- `42` (number) -> `Ok(42)`
- `"42"` (string) -> `Ok(42)`
- `" 42 "` (padded string) -> `Ok(42)`
- `"abc"` (invalid) -> `Err`
- `null` for optional fields -> `Ok(None)`

### Rollout Plan

1. Implement all four phases
2. Run `otto ci` to validate
3. `cargo install --path .` to update the local binary
4. Restart any running MCP server instances
5. Verify by exercising tools that previously errored

## Risks and Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| `run_raw()` tools lose structured output | Low | Medium | These tools never had JSON output; returning text is strictly better than erroring |
| `flexible_u64` accepts bad input silently | Low | Low | Still validates parsability; rejects non-numeric strings |
| Rate limit parsing breaks on `gh` output format changes | Medium | Low | Fallback to generic error if parsing fails; `extract_rate_limit_reset` returns "unknown" gracefully |
| Secondary rate limits (90s cooldown) have different error format | Medium | Low | Detection checks for both "rate limit exceeded" and "secondary rate limit" |
| `missing_scope` detection false positives | Low | Low | Only triggers on exact string match in stderr |

## Resolved Questions

- **`list_releases` implementation:** Use `gh release list --json` (not the raw API endpoint). It's consistent with how `list_repos` and `list_prs` already work, and `gh release list --json` is confirmed to support the fields we need.
- **`list_workflow_runs` limit type:** Yes, change its `limit` from `Option<u32>` to `Option<u64>` as part of Phase 4's type consistency sweep. The `--json` flag is already present on `list_workflow_runs`.

## References

- Session log analysis showing error frequencies (this conversation)
- `gh` CLI documentation: https://cli.github.com/manual
- GitHub REST API rate limiting: https://docs.github.com/en/rest/rate-limit
