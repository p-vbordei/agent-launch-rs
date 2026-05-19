//! Gather repo context: CHANGELOG section, README, recent commits, optional manifest.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Serialize;
use serde_json::Value;
use thiserror::Error;

pub const README_MAX: usize = 2000;

#[derive(Debug, Error)]
pub enum ContextError {
    #[error("{0}")]
    Missing(String),
    #[error("failed to parse manifest at {path}: {source}")]
    Manifest {
        path: String,
        #[source]
        source: serde_json::Error,
    },
}

#[derive(Debug, Clone, Serialize)]
pub struct GatheredContext {
    pub version: String,
    pub changelog: String,
    pub readme: String,
    pub commits: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manifest: Option<Value>,
}

pub fn gather_context(
    cwd: &Path,
    version: &str,
    manifest_path: Option<&Path>,
) -> Result<GatheredContext, ContextError> {
    let changelog_path = cwd.join("CHANGELOG.md");
    if !changelog_path.exists() {
        return Err(ContextError::Missing(format!(
            "CHANGELOG.md not found at {}",
            changelog_path.display()
        )));
    }
    let changelog_raw = std::fs::read_to_string(&changelog_path)
        .map_err(|e| ContextError::Missing(format!("failed to read CHANGELOG.md: {e}")))?;
    let changelog = extract_changelog_section(&changelog_raw, version)?;

    let readme_path = cwd.join("README.md");
    if !readme_path.exists() {
        return Err(ContextError::Missing(format!(
            "README.md not found at {}",
            readme_path.display()
        )));
    }
    let readme_full = std::fs::read_to_string(&readme_path)
        .map_err(|e| ContextError::Missing(format!("failed to read README.md: {e}")))?;
    let readme = if readme_full.len() > README_MAX {
        // Slice at the README_MAX byte; safe-truncate at char boundary if needed.
        let mut end = README_MAX;
        while !readme_full.is_char_boundary(end) && end > 0 {
            end -= 1;
        }
        readme_full[..end].to_string()
    } else {
        readme_full
    };

    let commits = read_recent_commits(cwd, 50);

    let manifest = if let Some(mp) = manifest_path {
        if !mp.exists() {
            return Err(ContextError::Missing(format!(
                "manifest file not found: {}",
                mp.display()
            )));
        }
        let raw = std::fs::read_to_string(mp)
            .map_err(|e| ContextError::Missing(format!("failed to read manifest: {e}")))?;
        let v = serde_json::from_str(&raw).map_err(|source| ContextError::Manifest {
            path: mp.display().to_string(),
            source,
        })?;
        Some(v)
    } else {
        None
    };

    Ok(GatheredContext {
        version: version.to_string(),
        changelog,
        readme,
        commits,
        manifest,
    })
}

fn extract_changelog_section(content: &str, version: &str) -> Result<String, ContextError> {
    // Find the `## [<version>] ...` heading line, then capture lines until the next `## [`
    // heading or end-of-file. We do this without lookahead since the `regex` crate doesn't
    // support it.
    let header = format!("## [{version}]");
    let mut start: Option<usize> = None;
    for (i, line) in content.lines().enumerate() {
        if line.starts_with(&header) {
            start = Some(i);
            break;
        }
    }
    let start = start.ok_or_else(|| {
        ContextError::Missing(format!("version {version} not found in CHANGELOG.md"))
    })?;
    let lines: Vec<&str> = content.lines().collect();
    let mut end = lines.len();
    for (i, line) in lines.iter().enumerate().skip(start + 1) {
        if line.starts_with("## [") {
            end = i;
            break;
        }
    }
    // Skip the header line itself.
    let body = lines[start + 1..end].join("\n");
    Ok(body.trim().to_string())
}

fn read_recent_commits(cwd: &Path, limit: usize) -> Vec<String> {
    let out = Command::new("git")
        .arg("log")
        .arg("--pretty=format:%h %s")
        .arg(format!("-{limit}"))
        .current_dir(cwd)
        .output();
    match out {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout)
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect(),
        _ => Vec::new(),
    }
}

#[allow(dead_code)]
fn _path_dummy() -> PathBuf {
    PathBuf::new()
}
