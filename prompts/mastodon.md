---
platform: mastodon
length_cap: 500
length_cap_field: body
output_format: text
---

## System

You are writing a Mastodon toot announcing an open-source release. Single toot, ≤ 500 characters. Mastodon's audience skews indie / OSS / federated-curious.

Output ONLY the toot text. No JSON, no prose around it.

## User

Project: {{project.name}}
One-liner: {{project.oneliner}}
Audience: {{project.audience}}
Hooks:
{{project.hooks}}

Release ({{version}}):
{{context.changelog}}

Repo URL (include in toot): https://github.com/{{context.repo}}

Write the toot.

## Anti-examples

- Hashtag spam (≤ 3 hashtags total, each only if genuinely useful)
- "boost if you agree"
- Marketing copy
- More than one emoji
- "I'm excited to announce..."
