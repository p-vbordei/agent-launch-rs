# Contributing

Thanks for your interest. This repo is a port of [`@p-vbordei/agent-launch`](https://github.com/p-vbordei/agent-launch); behaviour changes must land in the TS reference first so all ports can stay in sync.

## Setup

```bash
git clone https://github.com/p-vbordei/agent-launch-rs
cd agent-launch-rs
cargo build
```

## Run the tests

```bash
cargo test
```

47 tests should pass in under a second. No network or API key needed.

## Lint

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
```

## Run the offline example

```bash
cargo run --example quickstart
```

## Before opening a PR

- New behaviour must come with a test. Use the `Fake` `AnthropicClient` pattern in `tests/draft.rs` — no live API calls.
- Run `cargo fmt`, `cargo clippy -- -D warnings`, and `cargo test` locally. CI runs all three.
- Keep the diff scoped to the change. Don't reformat unrelated files.
- If your change affects the public API, update `CHANGELOG.md` under `## [Unreleased]`.

## Scope of changes

- **Prompts** (`prompts/*.md`) are vendored verbatim from the TS reference. Prompt changes must land there first; this repo updates by re-copying.
- **Behaviour deltas vs. the TS reference** are not accepted unless they fix a clear bug or close a gap in the spec. Open an issue first.
- Rust-idiomatic improvements (better error types, fewer allocations, clippy-clean refactors) are welcome as long as conformance + security tests still pass.

## License

By contributing, you agree your contributions are licensed under Apache-2.0 (see [LICENSE](./LICENSE)).
