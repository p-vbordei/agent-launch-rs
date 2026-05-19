# Architecture

## Goal

Port the TypeScript reference [`@p-vbordei/agent-launch`](https://github.com/p-vbordei/agent-launch) to idiomatic Rust while keeping the runtime behaviour identical: same `launch.yaml` schema, same prompt templates, same length caps, same conformance clauses (C1–C5, S2/S5).

"Identical" here means byte-identical generated drafts when given the same inputs and a `temperature=0` Anthropic client that returns the same text.

## Module map

| File | TS counterpart | Responsibility |
|---|---|---|
| `src/config.rs` | `src/config.ts` | Strict `launch.yaml` schema (serde `deny_unknown_fields` + manual validators). |
| `src/context.rs` | `src/context.ts` | Gather CHANGELOG section, README, recent `git log`, optional release manifest. |
| `src/platforms.rs` | `src/platforms.ts` | Read one of the five vendored prompt templates from `prompts/<kind>.md`. |
| `src/draft.rs` | `src/draft.ts` | Render the user template, call Anthropic, parse output, enforce length caps with up to 2 retries. |
| `src/bin/agent-launch.rs` | `src/index.ts` | clap-based CLI: `context` and `draft` subcommands. |
| `prompts/*.md` | `prompts/*.md` | Vendored verbatim from the TS reference. |

## Dependency choices

- **Anthropic client = trait, not a hard dep.** `draft.rs` defines `#[async_trait] pub trait AnthropicClient` with one method (`create(params: serde_json::Value) -> Result<Value, DraftError>`). The CLI binary supplies an HTTP impl (`reqwest`); tests and `examples/quickstart.rs` supply a `Mutex`-wrapped fake. Library callers bring their own transport.
- **YAML: `serde_yaml` with `deny_unknown_fields`.** Matches Zod's `.strict()` behaviour — extra fields are an error (C5). `deny_unknown_fields` is set on every config struct.
- **JSON: `serde_json` with `preserve_order`.** Output ordering matters for byte-identity with the TS reference.
- **CLI: `clap` (derive API).** The right level for a small two-subcommand CLI.
- **HTTP: `reqwest` + `rustls-tls`.** No OpenSSL pulls; binary stays self-contained. Only used in the binary, not in the library.
- **No native TLS / no OpenSSL.** Important for `cargo install` ergonomics on fresh boxes.

## Rust-specific gotcha: `regex` has no lookahead

The TS reference uses a regex with a lookahead (`(?=^## )`) to extract one section from a prompt template's body, stopping at the next `##` heading. The `regex` crate intentionally rejects lookarounds (linear-time guarantee).

We do **not** add `fancy-regex`. Instead `platforms::extract_section` and `context::extract_changelog_section` walk lines manually: find the start line, then scan forward to the next `## ` (or `## [`) heading or EOF. Same semantics, no extra dep, and the operation runs over short Markdown strings so the linear walk is fine.

## Testing strategy

- **Scripted Anthropic clients.** Every `draft_one` test (`tests/draft.rs`) injects a `Fake` impl of `AnthropicClient` that returns canned text and records the JSON `params` it received. No live API key; the full suite runs in `~1s`.
- **Conformance vectors.** `tests/conformance.rs` asserts the C1–C5 invariants directly against `draft_one`. Prompt-template byte-identity is enforced by shipping the TS-reference prompt files verbatim in `prompts/` (included in the crate via `Cargo.toml` `include`).
- **Security tests.** `tests/security.rs` covers S2 (no `tools` field passed to the Anthropic call) and S5 (`--out` sandboxing rejects absolute paths outside cwd and `..` traversal). S4 (no sockets) is covered implicitly: the library has no network code; only the binary's `HttpAnthropic` opens a socket.

## Non-goals (v0.1)

- Live posting. v0.1 writes markdown files only; the user posts them by hand. v0.2 will add post-on-approval for X/Mastodon/LinkedIn.
- Reply monitoring. v0.3.
- Multi-language translation, OG-image generation, analytics. Out of scope entirely.
