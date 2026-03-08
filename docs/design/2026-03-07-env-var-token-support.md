# Design Document: Environment Variable Token Support

**Author:** Scott Idler + Claude
**Date:** 2026-03-07
**Status:** Draft
**Ref:** [Secrets Standardization & Migration](~/docs/design/2026-03-07-secrets-standardization.md)

## Summary

Add `env:` prefix support to account token values so the MCP server can read tokens from environment variables instead of files. Follows the standard secrets pattern: `.age` files decrypt into env vars at shell init, apps read env vars.

## Problem

After secrets standardization, `~/.config/github/tokens/` files get removed. Tokens become env vars (`GITHUB_PAT_HOME`, `GITHUB_PAT_WORK`). The MCP server currently only reads from files — it needs to read from env vars too.

## Solution

Account values with an `env:` prefix read from environment variables. No prefix = existing file behavior.

```yaml
accounts:
  home: env:GITHUB_PAT_HOME
  work: env:GITHUB_PAT_WORK
  # file paths still work:
  # oss: ~/tokens/opensource
```

### What changes

| File | Change |
|------|--------|
| `src/error.rs` | Add `EnvVarNotFound(String)` variant |
| `src/config.rs` | `get_token()`: if source starts with `env:`, call `std::env::var()` instead of `fs::read_to_string()`. Rename `get_token_path()` → `get_token_source()`. |
| `src/main.rs` | `run_accounts()`: validate env vars (set + non-empty) instead of file-exists check |
| `multi-account-github-mcp.yml.example` | Add env var examples |

### Core logic

```rust
if let Some(var_name) = source.strip_prefix("env:") {
    let token = std::env::var(var_name)
        .map_err(|_| Error::EnvVarNotFound(var_name.to_string()))?
        .trim()
        .to_string();
    if token.is_empty() {
        return Err(Error::EnvVarNotFound(var_name.to_string()));
    }
    return Ok(token);
}
// else: existing file-based resolution
```

## Why `env:` prefix

- Backward compatible — no valid file path starts with `env:`
- No struct changes — `accounts: HashMap<String, String>` stays the same
- Explicit — no magic auto-detection or naming conventions
- Mixed configs work (some accounts env, some file)

## Alternatives rejected

- **Structured config (`type: env`)** — breaking change, over-engineered
- **Auto-detect by convention** — magic behavior, hard to debug
- **Inline tokens** — secrets in config files, violates the hard rule

## Testing

- `temp-env` crate (dev-dep) for safe env var manipulation in Rust 2024 edition
- Tests: var set → token returned, var missing → error, var empty → error, file path → unchanged

## Rollout

1. Implement, test, merge
2. Update config: `home: env:GITHUB_PAT_HOME`, `work: env:GITHUB_PAT_WORK`
3. Verify: `multi-account-github-mcp accounts` / `test home` / `test work`
4. Remove `~/.config/github/tokens/` (secrets standardization Phase 8)

## Risks

| Risk | Mitigation |
|------|------------|
| Env var not set at startup | Clear error naming the missing var |
| IDE launches without shell env | VS Code inherits shell env; verify during rollout |
