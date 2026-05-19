# Changelog

All notable changes to `agent-launch` (Rust) are documented in this file. Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/). Versioning follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2026-05-19

### Added

- Initial Rust port of [`@p-vbordei/agent-launch`](https://github.com/p-vbordei/agent-launch).
- 5 platform drafters: `hn`, `reddit`, `x`, `mastodon`, `linkedin`. Same prompts as the TS reference (vendored verbatim from `prompts/`).
- CLI binary: `agent-launch context <version>` and `agent-launch draft <version> [--platforms <list>] [--out <dir>]` (clap-based).
- Library API: `draft_one`, `gather_context`, `load_launch_config`, `AnthropicClient` trait, plus the `Platform`/`Project`/`GatheredContext`/`DraftResult` types.
- Byte-identical prompt templates with the TS reference. Prompts are shipped in the published crate via `Cargo.toml` `include`.
- Scripted-Anthropic-client test pattern: every `draft_one` test injects a `Fake` impl, no network, no API key. The HTTP-backed `AnthropicClient` lives in the binary only — the library has no network code.
- Strict `launch.yaml` schema (`serde` with `deny_unknown_fields` + manual validators for regex-shaped fields).
- Length-cap retry (up to 2 regenerations); over-cap drafts still written with `capped: false` in frontmatter.
- Output sandboxing: `--out` rejects absolute paths outside cwd and `..` traversal.
- Conformance vectors C1–C5 (determinism, length caps, repo URL present, no secrets, strict YAML) all pass.
- Security tests S2 (no `tools` field in the Anthropic call) and S5 (output sandbox).
- 47 tests across unit, integration, conformance, security suites.
- `examples/quickstart.rs` — offline 50-line walkthrough with a scripted `AnthropicClient`.
- GitHub Actions CI: `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`.
