//! Draft one platform-native release announcement via Claude.

use async_trait::async_trait;
use serde::Serialize;
use serde_json::{json, Map, Value};

use crate::config::{Platform, Project};
use crate::context::GatheredContext;
use crate::platforms::{
    load_platform_template, LengthCapField, OutputFormat, PlatformError, PlatformTemplate,
};

pub const MAX_RETRIES: usize = 2;
pub const MODEL: &str = "claude-opus-4-7";

/// Tiny adapter over the Anthropic messages API. Used so tests can supply a fake.
///
/// `params` is a JSON object with the same keys the SDK accepts:
/// `model`, `max_tokens`, `temperature`, `system`, `messages`.
/// Implementations must return a value with `content: [{type:"text", text:"..."}, ...]`.
#[async_trait]
pub trait AnthropicClient: Send + Sync {
    async fn create(&self, params: Value) -> Result<Value, DraftError>;
}

#[derive(Debug, thiserror::Error)]
pub enum DraftError {
    #[error("platform template error: {0}")]
    Template(#[from] PlatformError),
    #[error("anthropic API error: {0}")]
    Api(String),
}

#[derive(Debug, Clone, Serialize)]
pub struct DraftResult {
    pub platform: String,
    pub body: String,
    pub length: usize,
    pub length_cap: usize,
    pub capped: bool,
    pub retries: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tweet_count: Option<usize>,
}

#[derive(Debug, Clone)]
struct Parsed {
    body: String,
    title: Option<String>,
    tweet_count: Option<usize>,
}

pub async fn draft_one(
    platform: &Platform,
    project: &Project,
    context: &GatheredContext,
    repo: &str,
    anthropic: &dyn AnthropicClient,
) -> Result<DraftResult, DraftError> {
    let tpl = load_platform_template(platform.kind())?;
    let system_prompt = format!("{}\n\n## Anti-examples\n{}", tpl.system, tpl.anti_examples);
    let user_prompt = render_user_prompt(&tpl.user_template, platform, project, context, repo);

    let mut last: Option<Parsed> = None;
    let mut retries = 0usize;

    for attempt in 0..=MAX_RETRIES {
        let final_user = if attempt == 0 {
            user_prompt.clone()
        } else {
            let cap_clause = match tpl.length_cap_field {
                LengthCapField::PerTweet => {
                    format!("each tweet must be ≤ {} chars", tpl.length_cap)
                }
                LengthCapField::Body => {
                    format!("the body must be ≤ {} chars", tpl.length_cap)
                }
            };
            let title_clause = match tpl.title_cap {
                Some(cap) => format!(" Title ≤ {cap} chars."),
                None => String::new(),
            };
            format!(
                "{user_prompt}\n\nThe previous attempt exceeded the length limit. Rewrite shorter; {cap_clause}.{title_clause}"
            )
        };

        let mut params = Map::new();
        params.insert("model".into(), json!(MODEL));
        params.insert("max_tokens".into(), json!(4096));
        params.insert("temperature".into(), json!(0));
        params.insert("system".into(), json!(system_prompt));
        params.insert(
            "messages".into(),
            json!([{"role": "user", "content": final_user}]),
        );

        let resp = anthropic.create(Value::Object(params)).await?;
        let text = extract_text(&resp).trim().to_string();
        let parsed = parse_output(&tpl.output_format, &text);
        let ok = length_ok(&parsed, &tpl);
        retries = attempt;
        last = Some(parsed.clone());

        if ok {
            return Ok(make_result(platform.kind(), parsed, &tpl, true, retries));
        }
    }
    let last = last.expect("loop runs at least once");
    Ok(make_result(platform.kind(), last, &tpl, false, retries))
}

fn make_result(
    kind: &str,
    parsed: Parsed,
    tpl: &PlatformTemplate,
    capped: bool,
    retries: usize,
) -> DraftResult {
    let length = parsed.body.chars().count();
    DraftResult {
        platform: kind.to_string(),
        body: parsed.body,
        length,
        length_cap: tpl.length_cap,
        capped,
        retries,
        title: parsed.title,
        tweet_count: parsed.tweet_count,
    }
}

fn extract_text(resp: &Value) -> String {
    let Some(content) = resp.get("content").and_then(|c| c.as_array()) else {
        return String::new();
    };
    let mut out = String::new();
    for block in content {
        if block.get("type").and_then(|v| v.as_str()) == Some("text") {
            if let Some(t) = block.get("text").and_then(|v| v.as_str()) {
                out.push_str(t);
            }
        }
    }
    out
}

fn parse_output(fmt: &OutputFormat, text: &str) -> Parsed {
    match fmt {
        OutputFormat::Json => match serde_json::from_str::<Value>(text) {
            Ok(obj) => Parsed {
                body: obj
                    .get("body")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                title: Some(
                    obj.get("title")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                ),
                tweet_count: None,
            },
            Err(_) => Parsed {
                body: text.to_string(),
                title: None,
                tweet_count: None,
            },
        },
        OutputFormat::Thread => {
            let tweets: Vec<String> = text
                .split("---tweet---")
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            let count = tweets.len();
            Parsed {
                body: tweets.join("\n---tweet---\n"),
                title: None,
                tweet_count: Some(count),
            }
        }
        OutputFormat::Text => Parsed {
            body: text.to_string(),
            title: None,
            tweet_count: None,
        },
    }
}

fn length_ok(parsed: &Parsed, tpl: &PlatformTemplate) -> bool {
    if let (Some(cap), Some(title)) = (tpl.title_cap, parsed.title.as_ref()) {
        if title.chars().count() > cap {
            return false;
        }
    }
    match tpl.length_cap_field {
        LengthCapField::PerTweet => parsed
            .body
            .split("---tweet---")
            .map(|s| s.trim())
            .all(|t| t.chars().count() <= tpl.length_cap),
        LengthCapField::Body => parsed.body.chars().count() <= tpl.length_cap,
    }
}

fn render_user_prompt(
    template: &str,
    platform: &Platform,
    project: &Project,
    context: &GatheredContext,
    repo: &str,
) -> String {
    let hooks_lines = project
        .hooks
        .iter()
        .map(|h| format!("- {h}"))
        .collect::<Vec<_>>()
        .join("\n");
    let commits = context.commits.join("\n");
    let mut flat: Vec<(&str, String)> = vec![
        ("project.name", project.name.clone()),
        ("project.oneliner", project.oneliner.clone()),
        ("project.audience", project.audience.clone()),
        ("project.hooks", hooks_lines),
        ("context.changelog", context.changelog.clone()),
        ("context.readme", context.readme.clone()),
        ("context.commits", commits),
        ("context.repo", repo.to_string()),
        ("version", context.version.clone()),
    ];
    if let Platform::Reddit { subreddit } = platform {
        flat.push(("platform.subreddit", subreddit.clone()));
    }
    let mut out = template.to_string();
    for (k, v) in flat {
        out = out.replace(&format!("{{{{{k}}}}}"), &v);
    }
    out
}
