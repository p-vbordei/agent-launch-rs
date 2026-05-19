//! launch.yaml loader + strict deserialization.

use std::path::Path;

use regex::Regex;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum LaunchConfigError {
    #[error("failed to read or parse {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse {path}: {source}")]
    Yaml {
        path: String,
        #[source]
        source: serde_yaml::Error,
    },
    #[error("invalid launch.yaml: {0}")]
    Schema(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, tag = "kind", rename_all = "lowercase")]
pub enum Platform {
    Hn { pattern: HnPattern },
    Reddit { subreddit: String },
    X { handle: String },
    Mastodon { instance: String, handle: String },
    Linkedin,
}

impl Platform {
    pub fn kind(&self) -> &'static str {
        match self {
            Platform::Hn { .. } => "hn",
            Platform::Reddit { .. } => "reddit",
            Platform::X { .. } => "x",
            Platform::Mastodon { .. } => "mastodon",
            Platform::Linkedin => "linkedin",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum HnPattern {
    ShowHn,
    AskHn,
    Regular,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Project {
    pub name: String,
    pub oneliner: String,
    pub audience: String,
    pub hooks: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Context {
    pub repo: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manifest: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LaunchConfig {
    pub version: u8,
    pub project: Project,
    pub platforms: Vec<Platform>,
    pub context: Context,
}

/// Load a launch.yaml file and run strict schema validation.
pub fn load_launch_config(path: &Path) -> Result<LaunchConfig, LaunchConfigError> {
    let raw = std::fs::read_to_string(path).map_err(|source| LaunchConfigError::Io {
        path: path.display().to_string(),
        source,
    })?;
    let cfg: LaunchConfig =
        serde_yaml::from_str(&raw).map_err(|source| LaunchConfigError::Yaml {
            path: path.display().to_string(),
            source,
        })?;

    validate(&cfg)?;
    Ok(cfg)
}

fn validate(cfg: &LaunchConfig) -> Result<(), LaunchConfigError> {
    if cfg.version != 1 {
        return Err(LaunchConfigError::Schema(format!(
            "version: expected 1, got {}",
            cfg.version
        )));
    }
    let name_re = Regex::new(r"^[a-z0-9][a-z0-9-]*$").unwrap();
    if !name_re.is_match(&cfg.project.name) {
        return Err(LaunchConfigError::Schema(
            "project.name: must match /^[a-z0-9][a-z0-9-]*$/".into(),
        ));
    }
    if cfg.project.oneliner.is_empty() || cfg.project.oneliner.chars().count() > 120 {
        return Err(LaunchConfigError::Schema(
            "project.oneliner: 1..=120 chars".into(),
        ));
    }
    if cfg.project.audience.is_empty() || cfg.project.audience.chars().count() > 200 {
        return Err(LaunchConfigError::Schema(
            "project.audience: 1..=200 chars".into(),
        ));
    }
    if cfg.project.hooks.is_empty() {
        return Err(LaunchConfigError::Schema(
            "project.hooks: at least 1 required".into(),
        ));
    }
    if cfg.project.hooks.len() > 5 {
        return Err(LaunchConfigError::Schema(
            "project.hooks: at most 5 allowed".into(),
        ));
    }
    if cfg.project.hooks.iter().any(|h| h.is_empty()) {
        return Err(LaunchConfigError::Schema(
            "project.hooks: items must be non-empty".into(),
        ));
    }
    if cfg.platforms.is_empty() {
        return Err(LaunchConfigError::Schema(
            "platforms: at least 1 required".into(),
        ));
    }
    let repo_re = Regex::new(r#"^[^/]+/[^/]+$"#).unwrap();
    if !repo_re.is_match(&cfg.context.repo) {
        return Err(LaunchConfigError::Schema(
            "context.repo: must be \"owner/name\"".into(),
        ));
    }
    let subreddit_re = Regex::new(r"^[a-zA-Z0-9_]{2,21}$").unwrap();
    for p in &cfg.platforms {
        match p {
            Platform::Reddit { subreddit } => {
                if !subreddit_re.is_match(subreddit) {
                    return Err(LaunchConfigError::Schema(format!(
                        "reddit subreddit '{subreddit}' must match /^[a-zA-Z0-9_]{{2,21}}$/ (no leading r/)"
                    )));
                }
            }
            Platform::X { handle } => {
                if handle.is_empty() {
                    return Err(LaunchConfigError::Schema("x.handle: non-empty".into()));
                }
            }
            Platform::Mastodon { instance, handle } => {
                if instance.is_empty() {
                    return Err(LaunchConfigError::Schema(
                        "mastodon.instance: non-empty".into(),
                    ));
                }
                if handle.is_empty() {
                    return Err(LaunchConfigError::Schema(
                        "mastodon.handle: non-empty".into(),
                    ));
                }
            }
            _ => {}
        }
    }
    Ok(())
}
