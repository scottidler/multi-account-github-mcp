//! Observable effects of the binary's `setup_logging` and `run_test`.

use std::os::unix::fs::PermissionsExt;
use std::process::Command;

fn fake_gh_dir() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let script =
        "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then echo \"gh version 9.9.9\"; exit 0; fi\necho '\"fakeuser\"'\n";
    let path = dir.path().join("gh");
    std::fs::write(&path, script).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    dir
}

fn path_with(dir: &std::path::Path) -> String {
    format!("{}:{}", dir.display(), std::env::var("PATH").unwrap_or_default())
}

#[test]
fn test_subcommand_reports_gh_version_and_login() {
    let gh_dir = fake_gh_dir();
    let cfg_dir = tempfile::tempdir().unwrap();
    let cfg = cfg_dir.path().join("cfg.yml");
    std::fs::write(&cfg, "default_account: work\naccounts:\n  work: env:FAKE_TOKEN\n").unwrap();

    let out = Command::new(env!("CARGO_BIN_EXE_multi-account-github-mcp"))
        .env("PATH", path_with(gh_dir.path()))
        .env("FAKE_TOKEN", "tok")
        .args(["--config", cfg.to_str().unwrap(), "test"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Testing account: work"), "{stdout}");
    assert!(stdout.contains("gh version 9.9.9"), "{stdout}");
    assert!(stdout.contains("Authenticated as: fakeuser"), "{stdout}");
}

#[test]
fn logging_file_is_created_and_announced() {
    let dir = tempfile::tempdir().unwrap();
    let log = dir.path().join("nested/dir/server.log");
    let cfg = dir.path().join("cfg.yml");
    std::fs::write(
        &cfg,
        format!(
            "default_account: work\naccounts:\n  work: env:FAKE_TOKEN\nlogging:\n  level: info\n  file: {}\n",
            log.display()
        ),
    )
    .unwrap();

    let out = Command::new(env!("CARGO_BIN_EXE_multi-account-github-mcp"))
        .args(["--config", cfg.to_str().unwrap(), "accounts"])
        .output()
        .unwrap();
    assert!(out.status.success());
    assert!(log.exists(), "log file {} was not created", log.display());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("Logging to:"), "{stderr}");
}

#[test]
fn logging_file_in_unwritable_location_fails() {
    let dir = tempfile::tempdir().unwrap();
    let blocker = dir.path().join("blocker");
    std::fs::write(&blocker, "x").unwrap();
    let cfg = dir.path().join("cfg.yml");
    std::fs::write(
        &cfg,
        format!(
            "default_account: work\naccounts:\n  work: env:FAKE_TOKEN\nlogging:\n  file: {}/sub/server.log\n",
            blocker.display()
        ),
    )
    .unwrap();

    let out = Command::new(env!("CARGO_BIN_EXE_multi-account-github-mcp"))
        .args(["--config", cfg.to_str().unwrap(), "accounts"])
        .output()
        .unwrap();
    assert!(!out.status.success());
}
