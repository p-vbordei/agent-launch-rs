//! Conformance clauses C1–C5 from the agent-launch SPEC.

use std::fs;
use std::sync::Mutex;

use async_trait::async_trait;
use serde_json::{json, Value};
use tempfile::TempDir;

use agent_launch::config::{load_launch_config, HnPattern, LaunchConfigError, Platform, Project};
use agent_launch::context::GatheredContext;
use agent_launch::draft::{draft_one, AnthropicClient, DraftError};
use agent_launch::platforms::load_platform_template;

struct Fake(Mutex<Vec<String>>, Mutex<Option<Value>>);
impl Fake {
    fn new(texts: Vec<&str>) -> Self {
        Self(
            Mutex::new(texts.into_iter().map(|s| s.to_string()).collect()),
            Mutex::new(None),
        )
    }
}
#[async_trait]
impl AnthropicClient for Fake {
    async fn create(&self, params: Value) -> Result<Value, DraftError> {
        *self.1.lock().unwrap() = Some(params);
        let mut t = self.0.lock().unwrap();
        let text = if t.len() > 1 {
            t.remove(0)
        } else {
            t.last().cloned().unwrap_or_default()
        };
        Ok(json!({"content": [{"type": "text", "text": text}]}))
    }
}

fn project() -> Project {
    Project {
        name: "agent-id".into(),
        oneliner: "Machine-first identity for AI agents".into(),
        audience: "AI builders".into(),
        hooks: vec!["Self-custody DID".into(), "Three functions".into()],
    }
}

fn ctx() -> GatheredContext {
    GatheredContext {
        version: "0.2.0".into(),
        changelog: "### Added\n- thing".into(),
        readme: "# x".into(),
        commits: vec!["abc".into()],
        manifest: None,
    }
}

const REPO_URL: &str = "https://github.com/p-vbordei/agent-id";

// ---- C1 ----

#[tokio::test]
async fn c1_determinism() {
    let text = "Show HN: agent-id\n\nhttps://github.com/p-vbordei/agent-id";
    let fa = Fake::new(vec![text]);
    let fb = Fake::new(vec![text]);
    let a = draft_one(
        &Platform::Mastodon {
            instance: "h".into(),
            handle: "v".into(),
        },
        &project(),
        &ctx(),
        "p-vbordei/agent-id",
        &fa,
    )
    .await
    .unwrap();
    let b = draft_one(
        &Platform::Mastodon {
            instance: "h".into(),
            handle: "v".into(),
        },
        &project(),
        &ctx(),
        "p-vbordei/agent-id",
        &fb,
    )
    .await
    .unwrap();
    assert_eq!(a.body, b.body);
    assert_eq!(a.length, b.length);
    assert_eq!(a.capped, b.capped);
}

// ---- C2 ----

#[tokio::test]
async fn c2_within_cap_first_try() {
    let fake = Fake::new(vec!["short toot"]);
    let tpl = load_platform_template("mastodon").unwrap();
    let r = draft_one(
        &Platform::Mastodon {
            instance: "h".into(),
            handle: "v".into(),
        },
        &project(),
        &ctx(),
        "p-vbordei/agent-id",
        &fake,
    )
    .await
    .unwrap();
    assert!(r.body.chars().count() <= tpl.length_cap);
    assert!(r.capped);
    assert_eq!(r.retries, 0);
}

#[tokio::test]
async fn c2_over_then_within() {
    let long = "x".repeat(700);
    let fake = Fake::new(vec![&long, "short"]);
    let r = draft_one(
        &Platform::Mastodon {
            instance: "h".into(),
            handle: "v".into(),
        },
        &project(),
        &ctx(),
        "p-vbordei/agent-id",
        &fake,
    )
    .await
    .unwrap();
    assert!(r.capped);
    assert_eq!(r.retries, 1);
}

#[tokio::test]
async fn c2_over_forever() {
    let long = "x".repeat(700);
    let fake = Fake::new(vec![&long]);
    let r = draft_one(
        &Platform::Mastodon {
            instance: "h".into(),
            handle: "v".into(),
        },
        &project(),
        &ctx(),
        "p-vbordei/agent-id",
        &fake,
    )
    .await
    .unwrap();
    assert!(!r.capped);
    assert_eq!(r.retries, 2);
}

#[tokio::test]
async fn c2_x_per_tweet_cap() {
    let ok = ["tweet one", "tweet two has more", REPO_URL].join("\n---tweet---\n");
    let fake = Fake::new(vec![&ok]);
    let r = draft_one(
        &Platform::X { handle: "v".into() },
        &project(),
        &ctx(),
        "p-vbordei/agent-id",
        &fake,
    )
    .await
    .unwrap();
    assert!(r.capped);
    assert_eq!(r.tweet_count, Some(3));
}

// ---- C3 ----

#[tokio::test]
async fn c3_mastodon_repo_url() {
    let body = format!("look {REPO_URL}");
    let fake = Fake::new(vec![&body]);
    let r = draft_one(
        &Platform::Mastodon {
            instance: "h".into(),
            handle: "v".into(),
        },
        &project(),
        &ctx(),
        "p-vbordei/agent-id",
        &fake,
    )
    .await
    .unwrap();
    assert!(r.body.contains(REPO_URL));
}

#[tokio::test]
async fn c3_hn_repo_url() {
    let body = format!(r#"{{"title":"Show HN","body":"{REPO_URL}"}}"#);
    let fake = Fake::new(vec![&body]);
    let r = draft_one(
        &Platform::Hn {
            pattern: HnPattern::ShowHn,
        },
        &project(),
        &ctx(),
        "p-vbordei/agent-id",
        &fake,
    )
    .await
    .unwrap();
    assert!(r.body.contains(REPO_URL));
}

#[tokio::test]
async fn c3_x_repo_url() {
    let thread = format!("hook\n---tweet---\nbody\n---tweet---\n{REPO_URL}");
    let fake = Fake::new(vec![&thread]);
    let r = draft_one(
        &Platform::X { handle: "v".into() },
        &project(),
        &ctx(),
        "p-vbordei/agent-id",
        &fake,
    )
    .await
    .unwrap();
    assert!(r.body.contains(REPO_URL));
}

// ---- C4 ----

#[tokio::test]
async fn c4_no_secrets_in_body() {
    // Set fake secret envs; ensure body cannot contain them.
    // SAFETY: tests can race on process env, but values here don't appear in any prompt template.
    unsafe {
        std::env::set_var("ANTHROPIC_API_KEY_FAKE", "sk-ant-FAKE_TEST_TOKEN");
        std::env::set_var("GH_TOKEN_FAKE", "ghp_FAKE_TEST_TOKEN");
    }
    let fake = Fake::new(vec!["a clean post about agent-id"]);
    let r = draft_one(
        &Platform::Linkedin,
        &project(),
        &ctx(),
        "p-vbordei/agent-id",
        &fake,
    )
    .await
    .unwrap();
    assert!(!r.body.contains("sk-ant-"));
    assert!(!r.body.contains("ghp_"));
    assert!(!r.body.contains("npm_"));
}

#[tokio::test]
async fn c4_only_declared_fields() {
    let fake = Fake::new(vec!["short"]);
    let r = draft_one(
        &Platform::Mastodon {
            instance: "h".into(),
            handle: "v".into(),
        },
        &project(),
        &ctx(),
        "p-vbordei/agent-id",
        &fake,
    )
    .await
    .unwrap();
    let v = serde_json::to_value(&r).unwrap();
    let allowed: std::collections::HashSet<&str> = [
        "platform",
        "title",
        "body",
        "length",
        "length_cap",
        "capped",
        "retries",
        "tweet_count",
    ]
    .into_iter()
    .collect();
    for k in v.as_object().unwrap().keys() {
        assert!(allowed.contains(k.as_str()), "unexpected field: {k}");
    }
}

// ---- C5 ----

fn write_tmp(yaml: &str) -> (TempDir, std::path::PathBuf) {
    let d = TempDir::new().unwrap();
    let p = d.path().join("launch.yaml");
    fs::write(&p, yaml).unwrap();
    (d, p)
}

#[test]
fn c5_rejects_missing_version() {
    let bad = "project:\n  name: x\n  oneliner: y\n  audience: a\n  hooks: [\"a\"]\nplatforms:\n  - kind: linkedin\ncontext:\n  repo: o/r\n";
    let (_d, p) = write_tmp(bad);
    assert!(matches!(
        load_launch_config(&p).unwrap_err(),
        LaunchConfigError::Yaml { .. } | LaunchConfigError::Schema(_)
    ));
}

#[test]
fn c5_rejects_unknown_top_level_key() {
    let bad = "version: 1\nproject:\n  name: x\n  oneliner: y\n  audience: a\n  hooks: [\"a\"]\nplatforms:\n  - kind: linkedin\ncontext:\n  repo: o/r\nextra: y\n";
    let (_d, p) = write_tmp(bad);
    assert!(load_launch_config(&p).is_err());
}

#[test]
fn c5_rejects_unknown_platform_kind() {
    let bad = "version: 1\nproject:\n  name: x\n  oneliner: y\n  audience: a\n  hooks: [\"a\"]\nplatforms:\n  - kind: alien\ncontext:\n  repo: o/r\n";
    let (_d, p) = write_tmp(bad);
    assert!(load_launch_config(&p).is_err());
}

#[test]
fn c5_rejects_too_many_hooks() {
    let bad = "version: 1\nproject:\n  name: x\n  oneliner: y\n  audience: a\n  hooks: [\"a\",\"b\",\"c\",\"d\",\"e\",\"f\"]\nplatforms:\n  - kind: linkedin\ncontext:\n  repo: o/r\n";
    let (_d, p) = write_tmp(bad);
    let err = load_launch_config(&p).unwrap_err();
    assert!(matches!(err, LaunchConfigError::Schema(_)));
}

#[test]
fn c5_rejects_oneliner_too_long() {
    let long = "x".repeat(150);
    let bad = format!(
        "version: 1\nproject:\n  name: x\n  oneliner: \"{long}\"\n  audience: a\n  hooks: [\"a\"]\nplatforms:\n  - kind: linkedin\ncontext:\n  repo: o/r\n"
    );
    let (_d, p) = write_tmp(&bad);
    assert!(matches!(
        load_launch_config(&p).unwrap_err(),
        LaunchConfigError::Schema(_)
    ));
}
