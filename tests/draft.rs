use std::sync::Mutex;

use async_trait::async_trait;
use serde_json::{json, Value};

use agent_launch::config::{HnPattern, Platform, Project};
use agent_launch::context::GatheredContext;
use agent_launch::draft::{draft_one, AnthropicClient, DraftError};

struct Fake {
    texts: Mutex<Vec<String>>,
    captured: Mutex<Option<Value>>,
}

impl Fake {
    fn new(texts: Vec<&str>) -> Self {
        Self {
            texts: Mutex::new(texts.into_iter().map(|s| s.to_string()).collect()),
            captured: Mutex::new(None),
        }
    }
}

#[async_trait]
impl AnthropicClient for Fake {
    async fn create(&self, params: Value) -> Result<Value, DraftError> {
        *self.captured.lock().unwrap() = Some(params);
        let mut t = self.texts.lock().unwrap();
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
        changelog: "### Added\n- batch verify".into(),
        readme: "# agent-id\n\nDID for agents.".into(),
        commits: vec!["abc123 feat: batch verify".into()],
        manifest: None,
    }
}

#[tokio::test]
async fn hn_parses_json() {
    let fake = Fake::new(vec![
        r#"{"title":"Show HN: agent-id - DID for agents","body":"A short body. https://github.com/p-vbordei/agent-id"}"#,
    ]);
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
    assert_eq!(
        r.title.as_deref(),
        Some("Show HN: agent-id - DID for agents")
    );
    assert!(r.body.contains("https://github.com/p-vbordei/agent-id"));
    assert!(r.capped);
}

#[tokio::test]
async fn x_parses_thread() {
    let fake = Fake::new(vec![
        "Tweet 1 hook.\n---tweet---\nTweet 2 explains.\n---tweet---\nhttps://github.com/p-vbordei/agent-id",
    ]);
    let r = draft_one(
        &Platform::X {
            handle: "vbordei".into(),
        },
        &project(),
        &ctx(),
        "p-vbordei/agent-id",
        &fake,
    )
    .await
    .unwrap();
    assert!(r.body.contains("---tweet---"));
    assert_eq!(r.tweet_count, Some(3));
}

#[tokio::test]
async fn mastodon_returns_text() {
    let fake = Fake::new(vec!["Short toot. https://github.com/p-vbordei/agent-id"]);
    let r = draft_one(
        &Platform::Mastodon {
            instance: "hachyderm.io".into(),
            handle: "vbordei".into(),
        },
        &project(),
        &ctx(),
        "p-vbordei/agent-id",
        &fake,
    )
    .await
    .unwrap();
    assert!(r.body.contains("https://github.com/p-vbordei/agent-id"));
    assert!(r.title.is_none());
}

#[tokio::test]
async fn regenerates_when_over_cap() {
    let long = "a".repeat(700);
    let short = "short. https://github.com/p-vbordei/agent-id".to_string();
    let fake = Fake::new(vec![&long, &short]);
    let r = draft_one(
        &Platform::Mastodon {
            instance: "hachyderm.io".into(),
            handle: "vbordei".into(),
        },
        &project(),
        &ctx(),
        "p-vbordei/agent-id",
        &fake,
    )
    .await
    .unwrap();
    assert!(r.capped);
    assert_eq!(r.body, short);
    assert_eq!(r.retries, 1);
}

#[tokio::test]
async fn returns_over_length_after_retries() {
    let long = "a".repeat(700);
    let fake = Fake::new(vec![&long, &long, &long]);
    let r = draft_one(
        &Platform::Mastodon {
            instance: "hachyderm.io".into(),
            handle: "vbordei".into(),
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
async fn hn_enforces_title_cap() {
    let long_title = format!("Show HN: {}", "x".repeat(200));
    let long = format!(r#"{{"title":"{long_title}","body":"short body"}}"#,);
    let good = r#"{"title":"Show HN: agent-id - DID","body":"short. https://github.com/p-vbordei/agent-id"}"#.to_string();
    let fake = Fake::new(vec![&long, &good]);
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
    assert!(r.capped);
    assert!(r.title.unwrap().chars().count() <= 80);
}
