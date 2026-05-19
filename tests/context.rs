use std::fs;
use std::process::Command;

use tempfile::TempDir;

use agent_launch::context::{gather_context, ContextError};

fn git(args: &[&str], cwd: &std::path::Path) {
    let r = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .unwrap();
    if !r.status.success() {
        panic!("git {args:?}: {}", String::from_utf8_lossy(&r.stderr));
    }
}

fn setup_repo() -> TempDir {
    let d = TempDir::new().unwrap();
    git(&["init", "-b", "main"], d.path());
    git(&["config", "user.email", "t@example.com"], d.path());
    git(&["config", "user.name", "T"], d.path());
    fs::write(
        d.path().join("README.md"),
        "# agent-id\n\nMachine-first identity for AI agents.\n",
    )
    .unwrap();
    fs::write(
        d.path().join("CHANGELOG.md"),
        "# Changelog\n\n## [0.2.0] - 2026-04-27\n\n### Added\n- batch verification\n\n## [0.1.0] - 2026-04-20\n\n### Added\n- initial release\n",
    )
    .unwrap();
    git(&["add", "."], d.path());
    git(&["commit", "-m", "feat: batch verification"], d.path());
    fs::write(
        d.path().join("README.md"),
        "# agent-id\n\nMachine-first identity. Updated.\n",
    )
    .unwrap();
    git(&["add", "."], d.path());
    git(&["commit", "-m", "docs: update README"], d.path());
    d
}

#[test]
fn extracts_changelog_and_commits() {
    let d = setup_repo();
    let ctx = gather_context(d.path(), "0.2.0", None).unwrap();
    assert!(ctx.changelog.contains("batch verification"));
    assert!(!ctx.changelog.contains("## [0.1.0]"));
    assert!(ctx.readme.contains("Machine-first identity"));
    assert!(ctx.commits.len() >= 2);
    assert!(ctx.commits.iter().any(|c| c.contains("batch verification")));
}

#[test]
fn throws_on_missing_version() {
    let d = setup_repo();
    let err = gather_context(d.path(), "9.9.9", None).unwrap_err();
    assert!(matches!(err, ContextError::Missing(_)));
}

#[test]
fn throws_on_missing_readme() {
    let d = setup_repo();
    fs::remove_file(d.path().join("README.md")).unwrap();
    let err = gather_context(d.path(), "0.2.0", None).unwrap_err();
    assert!(matches!(err, ContextError::Missing(ref msg) if msg.contains("README")));
}

#[test]
fn truncates_readme_to_2000_chars() {
    let d = setup_repo();
    fs::write(d.path().join("README.md"), "a".repeat(5000)).unwrap();
    let ctx = gather_context(d.path(), "0.2.0", None).unwrap();
    assert!(ctx.readme.len() <= 2000);
}

#[test]
fn reads_optional_manifest() {
    let d = setup_repo();
    let manifest_path = d.path().join("release-manifest.json");
    fs::write(
        &manifest_path,
        r#"{"schema":"agent-publish/release-manifest/v1","version":"0.2.0"}"#,
    )
    .unwrap();
    let ctx = gather_context(d.path(), "0.2.0", Some(&manifest_path)).unwrap();
    let m = ctx.manifest.unwrap();
    assert_eq!(m.get("version").and_then(|v| v.as_str()), Some("0.2.0"));
}
