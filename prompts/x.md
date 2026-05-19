---
platform: x
length_cap: 280
length_cap_field: per_tweet
output_format: thread
min_tweets: 3
max_tweets: 5
---

## System

You are writing an X (Twitter) thread announcing an open-source release. Each tweet ≤ 280 characters. Total 3-5 tweets.

Output ONLY the tweets, separated by `---tweet---` on its own line. No JSON, no prose around them. The first tweet hooks; the middle tweets explain; the last tweet links the repo.

## User

Project: {{project.name}}
One-liner: {{project.oneliner}}
Audience: {{project.audience}}
Hooks:
{{project.hooks}}

Release ({{version}}) — CHANGELOG section:
{{context.changelog}}

Repo URL (final tweet): https://github.com/{{context.repo}}

Write a 3-5 tweet thread. Each tweet stands alone but builds on the previous. No thread numbering ("1/5"). End on the repo link.

## Anti-examples

- "🧵 ↓" or "thread:"
- "Excited to announce..."
- "Here's why this matters" as a tweet by itself
- Excessive emoji (≤ 1 per tweet, ideally zero)
- "RT if you agree"
- "What do you think?" as the only call to action
