//! Load per-platform prompt templates from `prompts/<kind>.md`.

use std::path::{Path, PathBuf};

use regex::Regex;
use serde::Deserialize;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlatformKind {
    Hn,
    Reddit,
    X,
    Mastodon,
    Linkedin,
}

impl PlatformKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            PlatformKind::Hn => "hn",
            PlatformKind::Reddit => "reddit",
            PlatformKind::X => "x",
            PlatformKind::Mastodon => "mastodon",
            PlatformKind::Linkedin => "linkedin",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "hn" => Some(PlatformKind::Hn),
            "reddit" => Some(PlatformKind::Reddit),
            "x" => Some(PlatformKind::X),
            "mastodon" => Some(PlatformKind::Mastodon),
            "linkedin" => Some(PlatformKind::Linkedin),
            _ => None,
        }
    }
}

#[derive(Debug, Error)]
pub enum PlatformError {
    #[error("unknown platform: {0}")]
    UnknownKind(String),
    #[error("platform template not found: {0}")]
    NotFound(String),
    #[error("invalid template (missing or malformed frontmatter): {0}")]
    BadFrontmatter(String),
    #[error("section \"{0}\" not found in template")]
    MissingSection(&'static str),
    #[error("yaml: {0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LengthCapField {
    Body,
    PerTweet,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputFormat {
    Json,
    Thread,
    Text,
}

#[derive(Debug, Clone, Deserialize)]
struct Frontmatter {
    platform: String,
    length_cap: usize,
    length_cap_field: LengthCapField,
    output_format: OutputFormat,
    #[serde(default)]
    title_cap: Option<usize>,
    #[serde(default)]
    min_tweets: Option<usize>,
    #[serde(default)]
    max_tweets: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct PlatformTemplate {
    pub platform: String,
    pub length_cap: usize,
    pub length_cap_field: LengthCapField,
    pub output_format: OutputFormat,
    pub title_cap: Option<usize>,
    pub min_tweets: Option<usize>,
    pub max_tweets: Option<usize>,
    pub system: String,
    pub user_template: String,
    pub anti_examples: String,
}

pub fn list_platforms() -> Vec<PlatformKind> {
    vec![
        PlatformKind::Hn,
        PlatformKind::Reddit,
        PlatformKind::X,
        PlatformKind::Mastodon,
        PlatformKind::Linkedin,
    ]
}

/// Locate the prompt file for a given kind. Walks up from CARGO_MANIFEST_DIR to find `prompts/`.
fn find_prompt(kind: PlatformKind) -> Result<PathBuf, PlatformError> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let candidate = manifest_dir
        .join("prompts")
        .join(format!("{}.md", kind.as_str()));
    if candidate.is_file() {
        return Ok(candidate);
    }
    // Fall back: walk up parents.
    let mut p: Option<&Path> = Some(&manifest_dir);
    while let Some(dir) = p {
        let c = dir.join("prompts").join(format!("{}.md", kind.as_str()));
        if c.is_file() {
            return Ok(c);
        }
        p = dir.parent();
    }
    Err(PlatformError::NotFound(kind.as_str().into()))
}

pub fn load_platform_template(kind: &str) -> Result<PlatformTemplate, PlatformError> {
    let k = PlatformKind::parse(kind).ok_or_else(|| PlatformError::UnknownKind(kind.into()))?;
    let path = find_prompt(k)?;
    let raw = std::fs::read_to_string(&path)?;
    let re = Regex::new(r"(?s)^---\n(.*?)\n---\n(.*)$").unwrap();
    let caps = re
        .captures(&raw)
        .ok_or_else(|| PlatformError::BadFrontmatter(path.display().to_string()))?;
    let fm_yaml = caps.get(1).unwrap().as_str();
    let body = caps.get(2).unwrap().as_str();
    let fm: Frontmatter = serde_yaml::from_str(fm_yaml)?;

    Ok(PlatformTemplate {
        platform: fm.platform,
        length_cap: fm.length_cap,
        length_cap_field: fm.length_cap_field,
        output_format: fm.output_format,
        title_cap: fm.title_cap,
        min_tweets: fm.min_tweets,
        max_tweets: fm.max_tweets,
        system: extract_section(body, "System")?,
        user_template: extract_section(body, "User")?,
        anti_examples: extract_section(body, "Anti-examples")?,
    })
}

fn extract_section(body: &str, heading: &'static str) -> Result<String, PlatformError> {
    // Find a line `## <heading>` and capture until the next `## ` or EOF.
    // `regex` crate has no lookahead, so we do this manually.
    let needle = format!("## {heading}");
    let lines: Vec<&str> = body.lines().collect();
    let start = lines
        .iter()
        .position(|l| *l == needle)
        .ok_or(PlatformError::MissingSection(heading))?;
    let mut end = lines.len();
    for (i, line) in lines.iter().enumerate().skip(start + 1) {
        if line.starts_with("## ") {
            end = i;
            break;
        }
    }
    let section = lines[start + 1..end].join("\n");
    Ok(section.trim().to_string())
}
