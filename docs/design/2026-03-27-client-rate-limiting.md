# Design Document: Client-Side Rate Limiting

**Author:** Scott Idler + Claude
**Date:** 2026-03-27
**Status:** Implemented
**Review Passes Completed:** 5/5

## Summary

Add per-account client-side rate limiting to `GhClient` using the `governor` crate. This proactively throttles outgoing `gh` CLI calls to stay under GitHub's API rate limits, preventing 403 errors before they happen. Rate limits are configurable via the YAML config file, defaulting to 80 req/min for general and 25 req/min for search endpoints.

## Problem Statement

### Background

The multi-account-github MCP server shells out to `gh` CLI for all GitHub API calls. Each GitHub account has independent rate limits: 5,000 req/hour primary, 90 req/min secondary (REST API burst), and 30 req/min for search endpoints. When these limits are hit, GitHub returns HTTP 403 errors.

### Problem

The previous error remediation work (Phases 2-3) improved detection and reporting of rate limit errors, but the server still hits the limits reactively. During heavy MCP usage, burst patterns can exhaust the secondary rate limit (90 req/min) quickly, causing cascading failures where multiple tool calls fail in rapid succession.

### Goals

- Proactively throttle requests to stay under GitHub's rate limits
- Per-account limiting since each account has independent quotas
- Separate limits for search vs general endpoints (search has lower limits)
- Configurable via the existing YAML config file with sensible binary defaults
- Transparent to callers - no API changes to `GhClient` methods

### Non-Goals

- Adaptive rate limiting based on response headers (would require parsing `gh` CLI output for headers)
- Queuing or request prioritization (callers await their turn equally)
- Per-tool rate limits (too granular, not needed)

## Proposed Solution

### Overview

Add a `RateLimitConfig` to `Config` and a per-account `RateLimiter` map to `GhClient`. Before every `run()`, `run_raw()`, or `api()` call, the client awaits the appropriate rate limiter. If the caller would exceed the limit, the await suspends the task until a token is available.

### Config Extension

Add a `rate-limit` section to the YAML config:

```yaml
default-account: home
accounts:
  home: ~/.config/github/tokens/personal
  work: env:GH_WORK_TOKEN

rate-limit:
  requests-per-minute: 80
  search-requests-per-minute: 25
```

All fields are optional with binary defaults:
- No `.yml` file at all - uses defaults (80 general, 25 search)
- `.yml` exists but no `rate-limit:` section - uses defaults
- `.yml` has `rate-limit:` with only one field - uses default for the missing field

**Implementation in `src/config.rs`:**

```rust
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct RateLimitConfig {
    #[serde(default = "default_requests_per_minute")]
    pub requests_per_minute: u32,

    #[serde(default = "default_search_requests_per_minute")]
    pub search_requests_per_minute: u32,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_minute: default_requests_per_minute(),
            search_requests_per_minute: default_search_requests_per_minute(),
        }
    }
}

fn default_requests_per_minute() -> u32 { 80 }
fn default_search_requests_per_minute() -> u32 { 25 }
```

Add to `Config`:
```rust
pub struct Config {
    // ...existing fields...
    #[serde(default, rename = "rate-limit")]
    pub rate_limit: RateLimitConfig,
}
```

The `Default` impl on `Config` must also include the new field:
```rust
impl Default for Config {
    fn default() -> Self {
        Self {
            // ...existing fields...
            rate_limit: RateLimitConfig::default(),
        }
    }
}
```

### Rate Limiter in GhClient

**Data structure:**

```rust
use governor::{Quota, RateLimiter as GovRateLimiter};
use governor::clock::DefaultClock;
use governor::state::{InMemoryState, NotKeyed};
use std::num::NonZeroU32;
use std::sync::Arc;
use dashmap::DashMap;

type Limiter = GovRateLimiter<NotKeyed, InMemoryState, DefaultClock>;

pub struct GhClient {
    config: Arc<Config>,
    general_limiters: Arc<DashMap<String, Arc<Limiter>>>,
    search_limiters: Arc<DashMap<String, Arc<Limiter>>>,
}
```

Using `DashMap` for concurrent access without explicit locking. Each account gets two limiters: one for general requests (caps total throughput), one for search (caps search-specific throughput).

**Limiter creation:**

```rust
fn make_limiter(requests_per_minute: u32) -> Arc<Limiter> {
    let rpm = requests_per_minute.max(1);
    let quota = Quota::per_minute(NonZeroU32::new(rpm).expect("rpm is at least 1"));
    Arc::new(GovRateLimiter::direct(quota))
}
```

Clamping to `max(1)` prevents panics if someone sets `0` in config. A rate limit of 1 req/min is effectively "disabled but not broken".

**Limiter resolution:**

```rust
fn get_limiter(&self, account: Option<&str>, is_search: bool) -> Arc<Limiter> {
    let key = account.unwrap_or(&self.config.default_account).to_string();
    let map = if is_search { &self.search_limiters } else { &self.general_limiters };
    let rpm = if is_search {
        self.config.rate_limit.search_requests_per_minute
    } else {
        self.config.rate_limit.requests_per_minute
    };
    map.entry(key).or_insert_with(|| make_limiter(rpm)).clone()
}
```

Limiters are lazily created on first request per account, per type.

**Search detection:**

A request is classified as search if `args[0] == "search"`. This covers both `search_prs` and `search_code` tool handlers, which build args as `["search", "prs"|"code", ...]`.

**Dual-limiter for search requests:**

GitHub counts search requests against BOTH the general secondary limit (90/min) and the search-specific limit (30/min). Therefore, search requests must pass through both limiters:

```rust
pub async fn run(&self, account: Option<&str>, args: &[&str]) -> Result<Value> {
    let is_search = args.first().is_some_and(|a| *a == "search");

    // All requests go through the general limiter
    let general = self.get_limiter(account, false);
    general.until_ready().await;

    // Search requests additionally go through the search limiter
    if is_search {
        let search = self.get_limiter(account, true);
        search.until_ready().await;
    }

    // ...existing implementation unchanged...
}
```

Same pattern for `run_raw()`. The `api()` method delegates to `run()` so it's automatically covered.

**Logging:** Add `tracing::debug!` when a request is throttled (i.e., when `until_ready()` would need to wait). Use `check()` first to detect whether we'd block, log if so, then `until_ready().await`:

```rust
let general = self.get_limiter(account, false);
if general.check().is_err() {
    tracing::debug!(
        account = account.unwrap_or("default"),
        command = %args.join(" "),
        "Rate limited - waiting for general quota"
    );
}
general.until_ready().await;
```

### Data Model

**New types:**

| Type | Location | Purpose |
|------|----------|---------|
| `RateLimitConfig` | `src/config.rs` | Deserializable config for rate limits |

**Modified types:**

| Type | Change |
|------|--------|
| `Config` | Add `rate_limit: RateLimitConfig` field |
| `GhClient` | Add `general_limiters` and `search_limiters` DashMap fields |

### Implementation Plan

| Phase | Description | Files Changed |
|-------|------------|---------------|
| 1 | Add dependencies, add `RateLimitConfig` to config | `Cargo.toml`, `src/config.rs` |
| 2 | Add rate limiters to `GhClient`, gate `run()`/`run_raw()` | `src/gh.rs` |

## Alternatives Considered

### Alternative 1: Single limiter per account (no search distinction)

- **Description:** One limiter at 80 req/min for all request types
- **Pros:** Simpler, one less config knob
- **Cons:** Search endpoints have a 30 req/min limit; a single 80 req/min limiter would still allow bursting past the search limit
- **Why not chosen:** The search rate limit is meaningfully lower. Two limiters cost almost nothing and prevent a real failure mode.

### Alternative 2: Mutex<HashMap> instead of DashMap

- **Description:** Use `std::sync::Mutex<HashMap<String, Arc<Limiter>>>` instead of DashMap
- **Pros:** No new dependency for the map
- **Cons:** Mutex contention under concurrent requests; lock held during HashMap operations
- **Why not chosen:** DashMap is purpose-built for this pattern. Contention is low but not zero - rmcp can invoke multiple tools concurrently. A `tokio::sync::RwLock<HashMap>` would also work but DashMap is more ergonomic.

### Alternative 3: Global (non-per-account) rate limiter

- **Description:** Single rate limiter shared across all accounts
- **Pros:** Simplest possible implementation
- **Cons:** Accounts have independent GitHub quotas; a single limiter unnecessarily throttles one account because another is busy
- **Why not chosen:** Per-account is the correct model since GitHub rate limits are per-token.

### Alternative 4: Adaptive rate limiting from X-RateLimit headers

- **Description:** Parse rate limit headers from GitHub API responses and dynamically adjust limits
- **Pros:** Perfectly tracks actual remaining quota
- **Cons:** We shell out to `gh` CLI which doesn't expose response headers; would require switching to direct HTTP or parsing `gh` debug output
- **Why not chosen:** Incompatible with our `gh` CLI architecture. The config-based approach is good enough.

## Technical Considerations

### Dependencies

New crate dependencies:
- `governor` - token bucket rate limiter (well-maintained, 7M+ downloads)
- `dashmap` - concurrent HashMap

```bash
cargo add governor dashmap
```

### Performance

- `until_ready().await` is essentially free when under the limit - it returns immediately
- When throttled, the task suspends (does not block a thread) until a token is available
- DashMap lookups are lock-free for reads after initial insertion
- Limiter creation happens once per account, per type (general/search)

### Security

No security implications. Rate limiting only throttles outgoing requests; it does not affect authentication or token handling.

### Testing Strategy

**Unit tests for config:**
- Parse YAML with `rate-limit` section - verify values
- Parse YAML without `rate-limit` section - verify defaults (80, 25)
- Parse YAML with partial `rate-limit` (only one field) - verify mixed defaults

**Unit tests for limiter management:**
- `get_limiter()` returns same limiter for same account
- `get_limiter()` returns different limiters for different accounts
- `get_limiter()` returns different limiters for search vs general on same account
- `make_limiter(0)` does not panic (clamped to 1)

### Rollout Plan

1. Implement both phases
2. Run `otto ci` to validate
3. `cargo install --path .` to update local binary
4. Restart MCP server
5. Monitor logs for rate-limit-related tracing messages

## Risks and Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| governor crate adds compile-time bloat | Low | Low | governor is lightweight; no heavy transitive dependencies |
| Default 80 rpm is too conservative for power users | Low | Low | Configurable via YAML; can raise to 89 (just under GitHub's 90) |
| Default 80 rpm is too aggressive | Low | Medium | GitHub's actual limit is 90; 80 gives 11% headroom |
| Limiter blocks legitimate burst patterns | Medium | Low | Token bucket naturally allows short bursts up to capacity; sustained throughput is what's limited |
| DashMap memory for many accounts | Low | Low | Typical usage is 1-3 accounts; each limiter is ~100 bytes |
| Config value of 0 causes panic | Low | High | Clamped to `max(1)` in `make_limiter()` |

## Resolved Questions

- **Should limiters persist across MCP server restarts?** No. Limiters are in-memory only. A fresh start means fresh quotas, which is correct since GitHub's rate limit window also resets.
- **Should we log when throttling occurs?** Yes, at `debug` level. Not `warn` because throttling is expected behavior, not an error.
- **Config field naming:** Use kebab-case (`rate-limit`, `requests-per-minute`) per project conventions; serde `rename_all = "kebab-case"` on `RateLimitConfig` and `rename = "rate-limit"` on the `Config` field.
- **Should search requests count against the general limiter too?** Yes. GitHub counts search requests against both the secondary burst limit and the search-specific limit. All requests go through the general limiter; search requests additionally go through the search limiter.
- **What if config sets 0 for a rate limit?** Clamped to 1 rpm in `make_limiter()`. This is effectively "almost disabled" without panicking.

## References

- GitHub REST API rate limits: https://docs.github.com/en/rest/rate-limit
- governor crate: https://docs.rs/governor
- Previous error remediation design: docs/design/2026-03-27-mcp-error-remediation.md
