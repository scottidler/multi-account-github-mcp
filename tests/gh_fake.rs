//! `GhClient` against a fake `gh` on PATH: the fake echoes its args and `GH_TOKEN`
//! so each assertion pins what the client actually passed and returned.

use multi_account_github_mcp::{Config, GhClient};
use std::collections::HashMap;
use std::os::unix::fs::PermissionsExt;

fn write_fake_gh(dir: &std::path::Path) {
    let script = r#"#!/bin/sh
if [ "$1" = "--version" ]; then echo "gh version 9.9.9"; exit 0; fi
if [ -n "$FAKE_GH_RAW" ]; then printf 'raw:%s|%s' "$*" "$GH_TOKEN"; exit 0; fi
printf '{"args":"%s","token":"%s"}' "$*" "$GH_TOKEN"
"#;
    let path = dir.join("gh");
    std::fs::write(&path, script).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
}

#[tokio::test]
async fn gh_client_passes_args_and_token_and_returns_output() {
    let dir = tempfile::tempdir().unwrap();
    write_fake_gh(dir.path());
    let old_path = std::env::var("PATH").unwrap_or_default();
    // Only test in this binary, so no other thread reads the environment concurrently.
    unsafe {
        std::env::set_var("PATH", format!("{}:{}", dir.path().display(), old_path));
        std::env::set_var("FAKE_GH_TOKEN", "tok-123");
    }

    let mut accounts = HashMap::new();
    accounts.insert("work".to_string(), "env:FAKE_GH_TOKEN".to_string());
    let config = Config {
        default_account: "work".to_string(),
        accounts,
        ..Config::default()
    };
    let gh = GhClient::new(config).unwrap();

    let value = gh.run(None, &["pr", "list"]).await.unwrap();
    assert_eq!(value["args"], "pr list");
    assert_eq!(value["token"], "tok-123");

    let value = gh
        .api(Some("work"), "repos/o/r", Some("POST"), Some(&[("k", "v")]))
        .await
        .unwrap();
    assert_eq!(value["args"], "api -X POST repos/o/r -f k=v");
    assert_eq!(value["token"], "tok-123");

    unsafe {
        std::env::set_var("FAKE_GH_RAW", "1");
    }
    let raw = gh.run_raw(None, &["pr", "diff", "7"]).await.unwrap();
    assert_eq!(raw, "raw:pr diff 7|tok-123");
}
