//! agent-launch CLI.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use async_trait::async_trait;
use chrono::Utc;
use clap::{Parser, Subcommand};
use serde_json::Value;

use agent_launch::config::{load_launch_config, LaunchConfigError, Platform};
use agent_launch::context::{gather_context, ContextError};
use agent_launch::draft::{draft_one, AnthropicClient, DraftError, DraftResult};

#[derive(Parser)]
#[command(
    name = "agent-launch",
    version,
    about = "Draft platform-native release announcements from CHANGELOG + README."
)]
struct Cli {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Print the gathered repo context as JSON.
    Context {
        /// Version that has a CHANGELOG section.
        version: String,
    },
    /// Draft platform-native release announcements.
    Draft {
        /// Version with a CHANGELOG section.
        version: String,
        /// Comma-separated platform kinds to render (default: all configured).
        #[arg(long)]
        platforms: Option<String>,
        /// Output dir (default: launches/v<version>). Must be inside cwd.
        #[arg(long)]
        out: Option<String>,
    },
}

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Cmd::Context { version } => run_context(&version).await,
        Cmd::Draft {
            version,
            platforms,
            out,
        } => run_draft(&version, platforms.as_deref(), out.as_deref()).await,
    }
}

async fn run_context(version: &str) -> ExitCode {
    let cwd = match std::env::current_dir() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("cwd: {e}");
            return ExitCode::from(1);
        }
    };
    let cfg = match load_cfg(&cwd) {
        Ok(c) => c,
        Err(code) => return code,
    };
    let manifest_path = cfg.context.manifest.as_ref().map(|m| cwd.join(m));
    let ctx = match gather_context(&cwd, version, manifest_path.as_deref()) {
        Ok(c) => c,
        Err(ContextError::Missing(msg)) => {
            eprintln!("{msg}");
            return ExitCode::from(1);
        }
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(1);
        }
    };
    match serde_json::to_string_pretty(&ctx) {
        Ok(s) => {
            println!("{s}");
            ExitCode::from(0)
        }
        Err(e) => {
            eprintln!("{e}");
            ExitCode::from(1)
        }
    }
}

async fn run_draft(version: &str, platforms: Option<&str>, out: Option<&str>) -> ExitCode {
    let cwd = match std::env::current_dir() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("cwd: {e}");
            return ExitCode::from(1);
        }
    };
    let cfg = match load_cfg(&cwd) {
        Ok(c) => c,
        Err(code) => return code,
    };

    let out_arg = out
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("launches/v{version}"));
    let out_abs = match resolve_under(&cwd, &out_arg) {
        Some(p) => p,
        None => {
            eprintln!("--out must resolve to a path inside {}", cwd.display());
            return ExitCode::from(1);
        }
    };

    let manifest_path = cfg.context.manifest.as_ref().map(|m| cwd.join(m));
    let ctx = match gather_context(&cwd, version, manifest_path.as_deref()) {
        Ok(c) => c,
        Err(ContextError::Missing(msg)) => {
            eprintln!("{msg}");
            return ExitCode::from(2);
        }
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(2);
        }
    };

    let api_key = match std::env::var("ANTHROPIC_API_KEY") {
        Ok(k) if !k.is_empty() => k,
        _ => {
            eprintln!("ANTHROPIC_API_KEY not set");
            return ExitCode::from(3);
        }
    };

    let filter: Option<std::collections::HashSet<String>> = platforms.map(|s| {
        s.split(',')
            .map(|x| x.trim().to_string())
            .collect::<std::collections::HashSet<_>>()
    });
    let targets: Vec<&Platform> = cfg
        .platforms
        .iter()
        .filter(|p| match &filter {
            None => true,
            Some(set) => set.contains(p.kind()),
        })
        .collect();
    if targets.is_empty() {
        eprintln!("no platforms match the --platforms filter");
        return ExitCode::from(1);
    }

    if let Err(e) = std::fs::create_dir_all(&out_abs) {
        eprintln!("create_dir_all {}: {e}", out_abs.display());
        return ExitCode::from(1);
    }

    let client = HttpAnthropic { api_key };
    let mut summary: Vec<(String, PathBuf, bool)> = Vec::new();
    for platform in targets {
        let result = match draft_one(platform, &cfg.project, &ctx, &cfg.context.repo, &client).await
        {
            Ok(r) => r,
            Err(DraftError::Api(msg)) => {
                eprintln!("Anthropic API: {msg}");
                return ExitCode::from(3);
            }
            Err(e) => {
                eprintln!("{e}");
                return ExitCode::from(1);
            }
        };
        let file = filename_for(platform, &out_abs);
        if let Err(e) = std::fs::write(&file, render_draft_file(&result)) {
            eprintln!("write {}: {e}", file.display());
            return ExitCode::from(1);
        }
        summary.push((platform.kind().to_string(), file, result.capped));
    }
    println!(
        "Drafted {} posts for v{} in {}",
        summary.len(),
        version,
        out_abs.display()
    );
    for (kind, file, capped) in &summary {
        let suffix = if *capped { "" } else { "  (over length cap)" };
        println!("  - {kind}: {}{suffix}", file.display());
    }
    ExitCode::from(0)
}

fn load_cfg(cwd: &Path) -> Result<agent_launch::config::LaunchConfig, ExitCode> {
    let path = cwd.join("launch.yaml");
    match load_launch_config(&path) {
        Ok(c) => Ok(c),
        Err(LaunchConfigError::Io { .. }) => {
            eprintln!("launch.yaml not found at {}", path.display());
            Err(ExitCode::from(1))
        }
        Err(e) => {
            eprintln!("{e}");
            Err(ExitCode::from(1))
        }
    }
}

fn resolve_under(cwd: &Path, arg: &str) -> Option<PathBuf> {
    let p = if Path::new(arg).is_absolute() {
        PathBuf::from(arg)
    } else {
        cwd.join(arg)
    };
    // Normalise: walk components manually so we don't need filesystem existence.
    let mut out = PathBuf::new();
    for c in p.components() {
        match c {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                if !out.pop() {
                    return None;
                }
            }
            other => out.push(other.as_os_str()),
        }
    }
    if out == *cwd || out.starts_with(cwd) {
        Some(out)
    } else {
        None
    }
}

fn filename_for(platform: &Platform, out_dir: &Path) -> PathBuf {
    match platform {
        Platform::Reddit { subreddit } => out_dir.join(format!("reddit-{subreddit}.md")),
        _ => out_dir.join(format!("{}.md", platform.kind())),
    }
}

fn render_draft_file(r: &DraftResult) -> String {
    let mut lines: Vec<String> = Vec::new();
    lines.push(format!("platform: {}", quote_if_needed(&r.platform)));
    lines.push(format!(
        "generated_at: {}",
        quote_if_needed(&Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string())
    ));
    lines.push(format!("length_chars: {}", r.length));
    lines.push(format!("length_cap: {}", r.length_cap));
    lines.push(format!("capped: {}", r.capped));
    if let Some(t) = &r.title {
        lines.push(format!("title: {}", json_str(t)));
    }
    if let Some(tc) = r.tweet_count {
        lines.push(format!("tweet_count: {tc}"));
    }
    format!("---\n{}\n---\n\n{}\n", lines.join("\n"), r.body)
}

fn quote_if_needed(s: &str) -> String {
    if s.chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        s.to_string()
    } else {
        json_str(s)
    }
}

fn json_str(s: &str) -> String {
    serde_json::to_string(s).unwrap_or_else(|_| format!("\"{s}\""))
}

// --- HTTP-backed Anthropic client (calls api.anthropic.com). ---

struct HttpAnthropic {
    api_key: String,
}

#[async_trait]
impl AnthropicClient for HttpAnthropic {
    async fn create(&self, params: Value) -> Result<Value, DraftError> {
        let client = reqwest::Client::builder()
            .build()
            .map_err(|e| DraftError::Api(e.to_string()))?;
        let resp = client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&params)
            .send()
            .await
            .map_err(|e| DraftError::Api(e.to_string()))?;
        let status = resp.status();
        let body: Value = resp
            .json()
            .await
            .map_err(|e| DraftError::Api(format!("response parse: {e}")))?;
        if !status.is_success() {
            return Err(DraftError::Api(format!("status {status}: {body}")));
        }
        Ok(body)
    }
}
