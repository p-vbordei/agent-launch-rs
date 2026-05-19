//! CLI integration tests.

use std::fs;
use std::process::Command;

use serde_json::Value;
use tempfile::TempDir;

fn cli_path() -> std::path::PathBuf {
    let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest.join("target").join("debug").join("agent-launch")
}

fn ensure_built() {
    if !cli_path().exists() {
        let r = Command::new("cargo")
            .args(["build", "--bin", "agent-launch"])
            .output()
            .expect("cargo build");
        assert!(
            r.status.success(),
            "cargo build failed: {}",
            String::from_utf8_lossy(&r.stderr)
        );
    }
}

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

fn setup_fixture() -> TempDir {
    let d = TempDir::new().unwrap();
    git(&["init", "-b", "main"], d.path());
    git(&["config", "user.email", "t@example.com"], d.path());
    git(&["config", "user.name", "T"], d.path());
    fs::write(
        d.path().join("README.md"),
        "# agent-id\n\nDID for agents.\n",
    )
    .unwrap();
    fs::write(
        d.path().join("CHANGELOG.md"),
        "# Changelog\n\n## [0.2.0] - 2026-04-27\n\n### Added\n- batch verify\n",
    )
    .unwrap();
    fs::write(
        d.path().join("launch.yaml"),
        "version: 1\nproject:\n  name: agent-id\n  oneliner: \"DID for agents\"\n  audience: \"AI builders\"\n  hooks:\n    - \"Self-custody\"\n    - \"Three functions\"\nplatforms:\n  - kind: hn\n    pattern: show-hn\ncontext:\n  repo: p-vbordei/agent-id\n",
    )
    .unwrap();
    git(&["add", "."], d.path());
    git(&["commit", "-m", "initial"], d.path());
    d
}

fn run(args: &[&str], cwd: &std::path::Path) -> (i32, String, String) {
    let out = Command::new(cli_path())
        .args(args)
        .current_dir(cwd)
        .env_remove("ANTHROPIC_API_KEY")
        .output()
        .expect("run");
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).to_string(),
        String::from_utf8_lossy(&out.stderr).to_string(),
    )
}

#[test]
fn context_prints_valid_json() {
    ensure_built();
    let d = setup_fixture();
    let (code, stdout, stderr) = run(&["context", "0.2.0"], d.path());
    assert_eq!(code, 0, "stderr: {stderr}");
    let v: Value = serde_json::from_str(&stdout).expect("valid JSON");
    assert_eq!(v.get("version").and_then(|x| x.as_str()), Some("0.2.0"));
    assert!(v
        .get("changelog")
        .and_then(|x| x.as_str())
        .unwrap()
        .contains("batch verify"));
    assert!(v
        .get("readme")
        .and_then(|x| x.as_str())
        .unwrap()
        .contains("DID for agents"));
    assert!(v.get("commits").and_then(|x| x.as_array()).is_some());
}

#[test]
fn exits_1_if_launch_yaml_missing() {
    ensure_built();
    let d = TempDir::new().unwrap();
    let (code, _stdout, _stderr) = run(&["context", "0.2.0"], d.path());
    assert_eq!(code, 1);
}

#[test]
fn exits_1_if_version_not_in_changelog() {
    ensure_built();
    let d = setup_fixture();
    let (code, _stdout, stderr) = run(&["context", "9.9.9"], d.path());
    assert_eq!(code, 1);
    assert!(stderr.contains("9.9.9"), "stderr: {stderr}");
}
