use std::fs;
use tempfile::TempDir;

use agent_launch::config::{load_launch_config, LaunchConfigError, Platform};

const VALID: &str = r#"version: 1
project:
  name: agent-id
  oneliner: "Machine-first identity for AI agents"
  audience: "AI infra builders"
  hooks:
    - "Self-custody DID + Capability VC"
    - "Three functions, five deps, zero blockchain"
platforms:
  - kind: hn
    pattern: show-hn
  - kind: reddit
    subreddit: programming
  - kind: reddit
    subreddit: typescript
  - kind: x
    handle: vbordei
  - kind: mastodon
    instance: hachyderm.io
    handle: vbordei
  - kind: linkedin
context:
  repo: p-vbordei/agent-id
"#;

fn write_tmp(yaml: &str) -> (TempDir, std::path::PathBuf) {
    let d = TempDir::new().unwrap();
    let p = d.path().join("launch.yaml");
    fs::write(&p, yaml).unwrap();
    (d, p)
}

#[test]
fn parses_valid_config() {
    let (_d, p) = write_tmp(VALID);
    let cfg = load_launch_config(&p).unwrap();
    assert_eq!(cfg.version, 1);
    assert_eq!(cfg.project.name, "agent-id");
    assert_eq!(cfg.project.hooks.len(), 2);
    assert_eq!(cfg.platforms.len(), 6);
    assert_eq!(cfg.context.repo, "p-vbordei/agent-id");
}

#[test]
fn allows_multiple_reddit_platforms() {
    let (_d, p) = write_tmp(VALID);
    let cfg = load_launch_config(&p).unwrap();
    let reddits: Vec<_> = cfg
        .platforms
        .iter()
        .filter(|p| matches!(p, Platform::Reddit { .. }))
        .collect();
    assert_eq!(reddits.len(), 2);
}

#[test]
fn rejects_invalid_platform_kind() {
    let bad = VALID.replace("  - kind: hn\n    pattern: show-hn\n", "  - kind: alien\n");
    let (_d, p) = write_tmp(&bad);
    assert!(load_launch_config(&p).is_err());
}

#[test]
fn rejects_invalid_hn_pattern() {
    let bad = VALID.replace("pattern: show-hn", "pattern: spam");
    let (_d, p) = write_tmp(&bad);
    assert!(load_launch_config(&p).is_err());
}

#[test]
fn rejects_invalid_subreddit() {
    let bad = VALID.replace("subreddit: programming", "subreddit: r/has-slash");
    let (_d, p) = write_tmp(&bad);
    let err = load_launch_config(&p).unwrap_err();
    assert!(matches!(err, LaunchConfigError::Schema(_)));
}

#[test]
fn rejects_extra_top_level_key() {
    let bad = format!("{VALID}extra: y\n");
    let (_d, p) = write_tmp(&bad);
    assert!(load_launch_config(&p).is_err());
}

#[test]
fn rejects_empty_hooks() {
    let bad = VALID.replace(
        "  hooks:\n    - \"Self-custody DID + Capability VC\"\n    - \"Three functions, five deps, zero blockchain\"\n",
        "  hooks: []\n",
    );
    let (_d, p) = write_tmp(&bad);
    assert!(load_launch_config(&p).is_err());
}

#[test]
fn rejects_more_than_5_hooks() {
    let six =
        "  hooks:\n    - \"a\"\n    - \"b\"\n    - \"c\"\n    - \"d\"\n    - \"e\"\n    - \"f\"\n";
    let bad = VALID.replace(
        "  hooks:\n    - \"Self-custody DID + Capability VC\"\n    - \"Three functions, five deps, zero blockchain\"\n",
        six,
    );
    let (_d, p) = write_tmp(&bad);
    assert!(load_launch_config(&p).is_err());
}

#[test]
fn rejects_malformed_repo() {
    let bad = VALID.replace("repo: p-vbordei/agent-id", "repo: norepo");
    let (_d, p) = write_tmp(&bad);
    assert!(load_launch_config(&p).is_err());
}

#[test]
fn manifest_is_optional() {
    let (_d, p) = write_tmp(VALID);
    let cfg = load_launch_config(&p).unwrap();
    assert!(cfg.context.manifest.is_none());

    let with_man = format!("{VALID}  manifest: ./release-manifest.json\n");
    let (_d2, p2) = write_tmp(&with_man);
    let cfg2 = load_launch_config(&p2).unwrap();
    assert_eq!(
        cfg2.context.manifest.as_deref(),
        Some("./release-manifest.json")
    );
}
