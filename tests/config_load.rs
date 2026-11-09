//! `Config::load` with an explicit path returns the file's values, not defaults.

use multi_account_github_mcp::Config;
use std::io::Write;

#[test]
fn load_explicit_path_returns_file_values() {
    let mut file = tempfile::NamedTempFile::new().unwrap();
    file.write_all(
        b"default_account: work\naccounts:\n  work: env:WORK_TOKEN\nlogging:\n  level: trace\nrate-limit:\n  requests-per-minute: 7\n",
    )
    .unwrap();

    let config = Config::load(Some(&file.path().to_path_buf())).unwrap();
    assert_eq!(config.default_account, "work");
    assert_eq!(config.get_token_source(None).unwrap(), "env:WORK_TOKEN");
    assert_eq!(config.logging.level, "trace");
    assert_eq!(config.rate_limit.requests_per_minute, 7);
}

#[test]
fn load_explicit_missing_path_is_an_error() {
    let missing = std::path::PathBuf::from("/nonexistent/dir/cfg.yml");
    assert!(Config::load(Some(&missing)).is_err());
}
