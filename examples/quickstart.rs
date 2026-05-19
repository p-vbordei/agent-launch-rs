//! Offline quickstart: draft a Show HN post with a scripted Anthropic client.
//!
//! Run: `cargo run --example quickstart`.
//! No network, no API key. The `AnthropicClient` is a stub that returns a
//! hardcoded JSON string — same shape the real SDK returns.

use std::sync::Mutex;

use agent_launch::{
    config::{HnPattern, Platform, Project},
    context::GatheredContext,
    draft::{draft_one, AnthropicClient, DraftError},
};
use async_trait::async_trait;
use serde_json::{json, Value};

struct ScriptedAnthropic {
    reply: String,
    last_params: Mutex<Option<Value>>,
}

#[async_trait]
impl AnthropicClient for ScriptedAnthropic {
    async fn create(&self, params: Value) -> Result<Value, DraftError> {
        *self.last_params.lock().unwrap() = Some(params);
        Ok(json!({"content": [{"type": "text", "text": &self.reply}]}))
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let reply = json!({
        "title": "Show HN: agent-launch — draft platform-native release posts",
        "body": "agent-launch turns your CHANGELOG + README into platform-tuned \
    release announcements for HN, Reddit, X, Mastodon, and LinkedIn. \
    v0.1 drafts only — you post. https://github.com/p-vbordei/agent-launch-rs"
    })
    .to_string();

    let client = ScriptedAnthropic {
        reply,
        last_params: Mutex::new(None),
    };

    let project = Project {
        name: "agent-launch".into(),
        oneliner: "Draft platform-native release announcements from CHANGELOG + README".into(),
        audience: "OSS maintainers shipping releases".into(),
        hooks: vec![
            "5 platforms".into(),
            "Strict YAML".into(),
            "Anti-slop prompts".into(),
        ],
    };
    let ctx = GatheredContext {
        version: "0.1.0".into(),
        changelog: "### Added\n- Initial Rust port.".into(),
        readme: "# agent-launch\n\nDraft release posts.".into(),
        commits: vec!["abc123 feat: initial port".into()],
        manifest: None,
    };

    let result = draft_one(
        &Platform::Hn {
            pattern: HnPattern::ShowHn,
        },
        &project,
        &ctx,
        "p-vbordei/agent-launch-rs",
        &client,
    )
    .await?;

    println!("platform : {}", result.platform);
    println!("title    : {}", result.title.as_deref().unwrap_or("<none>"));
    println!("length   : {} / cap {}", result.length, result.length_cap);
    println!("capped   : {}", result.capped);
    println!("retries  : {}", result.retries);
    println!("---");
    println!("{}", result.body);

    Ok(())
}
