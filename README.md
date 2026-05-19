# agent-launch (Rust)

[![CI](https://github.com/p-vbordei/agent-launch-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/p-vbordei/agent-launch-rs/actions/workflows/ci.yml)
[![Spec](https://img.shields.io/badge/spec-v0.1-blue)](./SPEC.md)
[![License](https://img.shields.io/badge/license-Apache%202.0-green)](./LICENSE)

> **Idiomatic Rust port of [@p-vbordei/agent-launch](https://www.npmjs.com/package/@p-vbordei/agent-launch).** Draft platform-native release announcements (HN, Reddit, X, Mastodon, LinkedIn) from your repo's CHANGELOG + README. v0.1: drafts only — you post. Output is **textually identical** to the TS reference when given the same inputs.

## What's in the box

- `draft_one(platform, project, context, repo, &client)` — produce a platform-tuned post draft
- Platforms: `hn`, `reddit`, `x`, `mastodon`, `linkedin`
- `gather_context(cwd, version, manifest)` — collect CHANGELOG section + README + recent commits
- `load_launch_config(path)` — strict serde-validated `launch.yaml`
- CLI binary: `agent-launch draft <version> [--platforms hn,x] [--out drafts/]`
- `AnthropicClient` trait — bring your own transport (HTTP, fake, mock)

## Install

```bash
cargo install agent-launch         # CLI binary
# or, as a library dependency:
cargo add agent-launch
```

## Quickstart

CLI (run inside any repo with a CHANGELOG.md, README.md, and `launch.yaml`):

```bash
cp launch.yaml.example launch.yaml && $EDITOR launch.yaml
ANTHROPIC_API_KEY=... agent-launch draft 0.2.0
# → launches/v0.2.0/{hn,reddit-*,x,mastodon,linkedin}.md
```

Library (offline — uses a scripted fake `AnthropicClient`):

```bash
cargo run --example quickstart
```

See [`examples/quickstart.rs`](examples/quickstart.rs) for the ~50-line walkthrough.

## How it relates

| Repo | Lang | Install |
|---|---|---|
| [`agent-launch`](https://github.com/p-vbordei/agent-launch) | TypeScript (reference) | `npm i -g @p-vbordei/agent-launch` |
| [`agent-launch-py`](https://github.com/p-vbordei/agent-launch-py) | Python | `pip install agent-launch` |
| [`agent-launch-rs`](https://github.com/p-vbordei/agent-launch-rs) | Rust (this repo) | `cargo install agent-launch` |

All three share the same `launch.yaml` schema, the same vendored prompt templates, and the same conformance clauses.

## Conformance

```bash
cargo test
```

The TS reference defines five conformance clauses (see [SPEC.md](./SPEC.md) §6). All pass here:

- **C1 — Determinism.** Same inputs → same output (with `temperature=0` and a scripted client, byte-identical).
- **C2 — Length caps.** Per-platform caps enforced with up to 2 retries; over-cap drafts marked `capped: false`.
- **C3 — Repo URL.** Every draft body contains the GitHub URL.
- **C4 — No secrets / declared fields only.** Env-var values are never echoed; `DraftResult` exposes only declared fields.
- **C5 — Strict `launch.yaml`.** Missing/extra/invalid fields fail fast with a clear error.

Plus security clauses **S2** (no `tools` field in the Anthropic call — model has no shell access) and **S5** (`--out` sandboxed under cwd).

## Architecture

See [docs/architecture.md](docs/architecture.md).

## Development

```bash
git clone https://github.com/p-vbordei/agent-launch-rs
cd agent-launch-rs
cargo test
cargo run --example quickstart
```

## License

Apache-2.0
