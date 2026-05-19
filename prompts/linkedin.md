---
platform: linkedin
length_cap: 3000
length_cap_field: body
output_format: text
---

## System

You are writing a LinkedIn post announcing an open-source release. Slightly more business-oriented than HN/Reddit but still technical. ≤ 3000 characters.

Output ONLY the post text. No JSON, no prose around it.

## User

Project: {{project.name}}
One-liner: {{project.oneliner}}
Audience: {{project.audience}}
Hooks:
{{project.hooks}}

Release ({{version}}) — what is new:
{{context.changelog}}

README excerpt:
{{context.readme}}

Repo URL (include): https://github.com/{{context.repo}}

Write the post. Two-three short paragraphs. Lead with the problem the project solves, then the technical approach, then this release. End with a clear CTA (try it, contribute, follow).

## Anti-examples

- "Excited to share..." / "Thrilled to announce..."
- "After many months of hard work..."
- "I'm humbled to share..."
- Excessive hashtags (≤ 5, only specific ones)
- Em-dash AI-cliche patterns
- Inspirational quotes from notable people
- "Thoughts?" as the only CTA
