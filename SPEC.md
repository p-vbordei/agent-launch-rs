# SPEC — agent-launch v0.1 (DRAFT)

**Status:** DRAFT 0.1 — flips to 1.0 at release per project scaffold.
**Last updated:** 2026-04-25

## 1. Overview

agent-launch is a single Bun-compiled binary. Given a `launch.yaml` and a project version, it gathers the relevant repo context (CHANGELOG section, README, recent commits, optional release manifest) and produces platform-native announcement drafts in `launches/v<version>/`. v0.1 is draft-only — no API posting.

## 2. Data model

### 2.1 `launch.yaml` (v0.1, strict)

```yaml
version: 1
project:
  name: <string>            # matches /^[a-z0-9][a-z0-9-]*$/
  oneliner: <string>        # ≤ 120 chars
  audience: <string>        # ≤ 200 chars; informs voice
  hooks:                    # 1-5 items; specific, technical, used in headlines
    - <string>
    - ...
platforms:
  - kind: <string>          # one of: hn, reddit, x, mastodon, linkedin
    # kind-specific keys below; strict
context:
  repo: <string>            # GitHub coordinate "owner/name"
  manifest: <string>        # optional; path to release manifest JSON
```

Per-platform keys:

```yaml
- kind: hn
  pattern: <string>         # one of: show-hn, ask-hn, regular
- kind: reddit
  subreddit: <string>       # without leading r/; matches /^[a-zA-Z0-9_]{2,21}$/
- kind: x
  handle: <string>          # informational in v0.1
- kind: mastodon
  instance: <string>        # FQDN
  handle: <string>
- kind: linkedin
  # no required keys
```

Multiple `reddit` platforms permitted (one per subreddit). Other kinds appear at most once.

### 2.2 Output: `launches/v<version>/<kind>[.<discriminator>].md`

For HN: `hn.md`. For Reddit: `reddit-<subreddit>.md`. For X: `x.md`. For Mastodon: `mastodon.md`. For LinkedIn: `linkedin.md`.

Each file is markdown with a YAML frontmatter:

```markdown
---
platform: hn
pattern: show-hn
generated_at: 2026-04-25T13:30:00Z
length_chars: 1842
length_cap: 2000
title: "Show HN: agent-id – self-custody DID + capability VC for AI agents"
url: https://news.ycombinator.com/submit
---

<body markdown>
```

Multi-tweet X threads: body is `---tweet---` separated.

## 3. CLI

```
agent-launch draft <version> [--platforms <list>] [--out <dir>]
agent-launch context [--version <version>]
```

### 3.1 `draft`

**Inputs:**
- `<version>` (required): semver string, must match a `## [<version>]` heading in CHANGELOG.md
- `--platforms` (optional): comma-separated list of platform kinds to render (default: all configured)
- `--out` (optional): output dir (default: `launches/v<version>/`)

**Behavior:**
1. Load and validate `launch.yaml`.
2. Gather context:
   - Extract CHANGELOG section for the version
   - Read README.md
   - List last 50 commits since the previous tag (`git log` if local, else `gh api`)
   - If `context.manifest` is set and the file exists, parse it
3. For each configured platform (or filtered set):
   a. Load `prompts/<kind>.md` (vendored)
   b. Render with project + context
   c. Call Claude (`claude-opus-4-7`, temperature 0)
   d. Validate length against platform cap
   e. If over cap and retries < 2, regenerate with explicit "shorter, ≤ N chars" instruction
   f. If still over cap after retries, write anyway with a warning in stderr
   g. Write to output dir with frontmatter
4. Print summary: `Drafted N posts for v<version> in <dir>` + bullet list.

**Exit codes:**
- 0 success
- 1 launch.yaml invalid or version not found in CHANGELOG
- 2 missing CHANGELOG / README / repo context
- 3 Claude API error after retry
- 4 unrecoverable length-cap violation across all retries (still writes the file but exits non-zero)

### 3.2 `context`

Prints the gathered context blob (JSON) without calling Claude. Used for debugging launch.yaml.

## 4. Platform constraints (v0.1, normative)

| kind | Format | Length cap | Voice notes |
|---|---|---|---|
| `hn` | title + body | title ≤ 80 chars, body ≤ 2000 chars | Direct, no hype, technical, links repo |
| `reddit` | title + body | title ≤ 300 chars, body ≤ 5000 chars | Subreddit-aware (technical for r/programming, ecosystem-fit for r/typescript) |
| `x` | thread, 3-5 tweets, separated by `---tweet---` | each ≤ 280 chars | Hooky first tweet, technical body, link last |
| `mastodon` | single toot | ≤ 500 chars (configurable per instance via `instance_chars`, deferred) | Indie / OSS / federated voice |
| `linkedin` | post | ≤ 3000 chars | Slightly more business voice, but still technical |

These caps are HARD: drafts exceeding them after 2 retries trigger exit code 4.

## 5. Prompt template structure (normative)

Each `prompts/<kind>.md` file follows this structure:

```markdown
---
platform: <kind>
length_cap: <number>
length_cap_field: <e.g. "title", "body", "tweet">  # which field has the cap
---

# System prompt

<the system prompt — defines voice, what NOT to do>

# User prompt

<the user prompt template, with Mustache vars: {{project.name}}, {{project.oneliner}},
{{project.audience}}, {{project.hooks}}, {{context.changelog}}, {{context.readme_summary}},
{{context.recent_commits}}, {{context.manifest}}>

# Anti-examples

<hand-curated list of phrases / patterns to avoid: "revolutionary", "game-changing", em dashes
in the AI-cliche pattern, etc.>
```

The anti-examples block is part of the system prompt at runtime — it instructs the model NOT to produce certain marketing-AI tropes.

## 6. Conformance clauses

- **C1 — Determinism (best-effort).** Running `agent-launch draft <version>` twice with identical inputs (launch.yaml, CHANGELOG, README, commits, manifest) and `temperature: 0` produces drafts that match within ≤ 1% character difference per file. (Strict byte-equality is best-effort; some model nondeterminism is accepted up to this bound.)
- **C2 — Length caps respected.** Across N=10 fixture runs, ≥ 90% of drafts are within the platform cap on first try; ≥ 99% within 2 retries.
- **C3 — Repo URL present.** Every draft contains at least one URL whose host is `github.com` and whose path starts with `<repo>`.
- **C4 — No secrets / no PII beyond declared.** Drafts contain no environment variable values, no `ANTHROPIC_API_KEY` echo, and only fields declared in `launch.yaml`.
- **C5 — launch.yaml strict.** Loading a launch.yaml with missing/extra/invalid fields fails fast with a non-zero exit and a clear error message.

A test in `conformance/` validates each clause with fixtures. Full conformance run completes in < 30 seconds (drafts use cached fixture context to avoid live API calls in CI).

## 7. Security considerations

- **S1 — Secrets.** `ANTHROPIC_API_KEY` is read from env only. Never logged. Never written to disk. Never appears in drafts.
- **S2 — Tool surface.** v0.1 has NO bash/shell tool exposed to the model — drafts are pure-text generation. Context gathering happens in agent-launch's own code, not in the model.
- **S3 — Prompt injection.** Prompt templates are vendored at agent-launch HEAD, never user-editable beyond `launch.yaml` declared fields. CHANGELOG / README content IS user-controlled (they wrote it), so injection is the user injecting into their own draft — out of threat model.
- **S4 — No network writes.** v0.1 does not POST anywhere. The only network operations are: Claude API calls, optional `gh api` for commits / README on private repos. Validated by integration test: spy on fetch, assert no posts to social platforms.
- **S5 — Output sandboxing.** `--out` is restricted to a path within the current working directory. Absolute paths or `..` traversal rejected.

## 8. Versioning

- agent-launch itself follows semver.
- `launch.yaml` schema is versioned in the `version: 1` field.
- Prompt templates are vendored per agent-launch release. Users adopt newer voice by upgrading.

## 9. Deliverables checklist (Stage 6)

- [ ] `bun install && bun test` green on a clean checkout
- [ ] `bun build --compile --outfile agent-launch src/index.ts` produces a single binary
- [ ] `examples/demo.ts` produces 5 drafts for agent-id v0.1.0 in `launches/v0.1.0/`
- [ ] All conformance clauses pass
- [ ] CHANGELOG.md v0.1.0 entry
- [ ] SPEC.md banner flipped DRAFT → 1.0 (at v1.0, NOT at v0.1)
- [ ] Git tag v0.1.0 created locally; push deferred to user confirmation
