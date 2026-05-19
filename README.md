# agent-launch (Rust)

[![CI](https://github.com/p-vbordei/agent-launch-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/p-vbordei/agent-launch-rs/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/agent-launch.svg)](https://crates.io/crates/agent-launch)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](./LICENSE)

> Draft platform-native release announcements (HN, Reddit, X, Mastodon, LinkedIn) from your repo's CHANGELOG + README + recent commits. **v0.1: drafts only — you post.**

Rust port of [@p-vbordei/agent-launch](https://github.com/p-vbordei/agent-launch). Behaviour-compatible: same `launch.yaml` schema, same prompt templates, same per-platform length caps, same conformance clauses (C1–C5, S2/S5).

## Install

```bash
cargo install agent-launch
```

## Use

```bash
cp launch.yaml.example launch.yaml && $EDITOR launch.yaml
agent-launch context 0.2.0                        # debug: print the gathered context
ANTHROPIC_API_KEY=... agent-launch draft 0.2.0    # → launches/v0.2.0/{hn,reddit-*,x,mastodon,linkedin}.md
agent-launch draft 0.2.0 --platforms hn,x         # only specific platforms
agent-launch draft 0.2.0 --out drafts/            # custom output dir (must be inside cwd)
```

Each draft is platform-native: HN's "Show HN" pattern, Reddit's subreddit etiquette, X's threaded format, Mastodon's longer-form toot, LinkedIn's slightly-more-business voice. Anti-examples (`revolutionary`, `game-changing`, the em-dash AI-cliche pattern, etc.) are baked into every prompt.

## Library

```rust
use std::path::Path;
use agent_launch::{draft_one, gather_context, load_launch_config};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cfg = load_launch_config(Path::new("launch.yaml"))?;
    let ctx = gather_context(Path::new("."), "0.2.0", None)?;
    // Bring your own `impl AnthropicClient`; or use the built-in HTTP one in the binary.
    Ok(())
}
```

## Conformance

The TypeScript reference defines five conformance clauses (see [SPEC.md](./SPEC.md) §6). All pass here:

- **C1 — Determinism.** Same inputs → same output (with `temperature=0` and a scripted client, byte-identical).
- **C2 — Length caps.** Per-platform caps enforced with up to 2 retries; over-cap drafts are still written but marked `capped: false`.
- **C3 — Repo URL.** Every draft body contains the GitHub URL.
- **C4 — No secrets / declared fields only.** Env-var values are never echoed; `DraftResult` exposes only declared fields.
- **C5 — Strict `launch.yaml`.** Missing/extra/invalid fields fail fast with a clear error.

Plus security clauses **S2** (no `tools` field in the Anthropic call — model has no shell access) and **S5** (`--out` sandboxed under cwd).

## License

[Apache 2.0](./LICENSE)
