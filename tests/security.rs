//! Security clauses S2 (no tools in request) and S5 (sandboxed --out).

use std::fs;
use std::process::Command;
use std::sync::Mutex;

use async_trait::async_trait;
use serde_json::{json, Value};
use tempfile::TempDir;

use agent_launch::config::{Platform, Project};
use agent_launch::context::GatheredContext;
use agent_launch::draft::{draft_one, AnthropicClient, DraftError};

struct Capture(Mutex<Option<Value>>);
#[async_trait]
impl AnthropicClient for Capture {
    async fn create(&self, params: Value) -> Result<Value, DraftError> {
        *self.0.lock().unwrap() = Some(params);
        Ok(json!({"content": [{"type": "text", "text": "short text"}]}))
    }
}

fn project() -> Project {
    Project {
        name: "x".into(),
        oneliner: "y".into(),
        audience: "a".into(),
        hooks: vec!["h".into()],
    }
}
fn ctx() -> GatheredContext {
    GatheredContext {
        version: "0.2.0".into(),
        changelog: "c".into(),
        readme: "r".into(),
        commits: vec!["c".into()],
        manifest: None,
    }
}

#[tokio::test]
async fn s2_no_tools_field_in_params() {
    let cap = Capture(Mutex::new(None));
    draft_one(
        &Platform::Mastodon {
            instance: "h".into(),
            handle: "v".into(),
        },
        &project(),
        &ctx(),
        "o/r",
        &cap,
    )
    .await
    .unwrap();
    let p = cap.0.lock().unwrap().clone().unwrap();
    let obj = p.as_object().unwrap();
    assert!(!obj.contains_key("tools"), "tools must NEVER be in params");
    assert_eq!(
        obj.get("model").and_then(|v| v.as_str()),
        Some("claude-opus-4-7")
    );
    assert!(obj.get("system").and_then(|v| v.as_str()).is_some());
}

// ---- S5 — sandboxed --out via CLI ----

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
    fs::write(d.path().join("README.md"), "# x\n").unwrap();
    fs::write(
        d.path().join("CHANGELOG.md"),
        "## [0.2.0]\n\n### Added\n- thing\n",
    )
    .unwrap();
    fs::write(
        d.path().join("launch.yaml"),
        "version: 1\nproject:\n  name: x\n  oneliner: y\n  audience: a\n  hooks: [\"a\"]\nplatforms:\n  - kind: linkedin\ncontext:\n  repo: o/r\n",
    )
    .unwrap();
    git(&["add", "."], d.path());
    git(&["commit", "-m", "i"], d.path());
    d
}

fn cli_path() -> std::path::PathBuf {
    // `cargo test` builds the bin into target/debug/agent-launch.
    let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest.join("target").join("debug").join("agent-launch")
}

fn run_cli(args: &[&str], cwd: &std::path::Path) -> (i32, String, String) {
    let out = Command::new(cli_path())
        .args(args)
        .current_dir(cwd)
        .env_remove("ANTHROPIC_API_KEY")
        .output()
        .expect("run cli");
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).to_string(),
        String::from_utf8_lossy(&out.stderr).to_string(),
    )
}

#[test]
fn s5_rejects_absolute_out_outside_cwd() {
    if !cli_path().exists() {
        // Build the binary first.
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
    let d = setup_fixture();
    let (code, _stdout, stderr) = run_cli(&["draft", "0.2.0", "--out", "/tmp/escape"], d.path());
    assert_eq!(code, 1);
    assert!(stderr.contains("inside"), "stderr was: {stderr}");
}

#[test]
fn s5_rejects_dotdot_traversal() {
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
    let d = setup_fixture();
    let (code, _stdout, stderr) = run_cli(&["draft", "0.2.0", "--out", "../escape"], d.path());
    assert_eq!(code, 1);
    assert!(stderr.contains("inside"), "stderr was: {stderr}");
}
