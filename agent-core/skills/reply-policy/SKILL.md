---
name: reply-policy
description: Gate outbound replies with templates and a safety guard.
version: 0.1.0
author: Office Agent
platforms: [linux, macos, windows]
metadata:
  hermes:
    tags: [messaging, policy, moderation, hr, social]
    category: communication
---

# Reply Policy Skill

Approve, queue, or drop outbound replies for social-media and HR messaging.

## When to Use

Use this skill when the agent drafts replies to public comments, direct
messages, or Zalo Official Account inbound messages. It enforces a
configurable reply policy: match an allow-listed template, run a lightweight
LLM guard, and decide whether to `send`, `queue_human`, or `drop`.

## Configuration

Set in `config.yaml` under `skills.config.reply_policy`:

- `mode` — `"auto"` | `"suggest"` | `"off"`. Default `"suggest"`.
  - `auto`: send if template matches and guard passes.
  - `suggest`: always queue for human review with a proposed reply.
  - `off`: drop all outbound replies.
- `templates_path` — path to `templates.toml`. Defaults to
  `<skill_dir>/templates.toml`.

## How to Run

Import the shared module from other skills or MCP handlers. Because the
skill directory name contains a hyphen, add it to `sys.path` and import the
module directly:

```python
import sys
sys.path.insert(0, "/path/to/agent-core/skills/reply-policy")
from policy import load_templates, decide_reply

templates = load_templates(templates_path)
result = decide_reply(
    inbound="Cảm ơn shop nhé",
    platform="youtube",
    scenario="youtube_thank_you",
    mode="auto",
    templates=templates,
    context={},
)
# result["action"] in {"send", "queue_human", "drop"}
```

## Quick Reference

| Function | Purpose |
| --- | --- |
| `load_templates(path)` | Load approved templates from TOML. |
| `match_template(platform, scenario, inbound_text, templates)` | Return the matching template or `None`. |
| `llm_guard(reply_text, context)` | Lightweight rule-based guard. |
| `decide_reply(...)` | Decide action, reply, and reason. |

## Procedure

1. Match `(platform, scenario)` against `templates.toml`.
2. Render the template text (callers may substitute placeholders before
   passing to `decide_reply`).
3. If `mode` is `off`, return `drop`.
4. If `mode` is `suggest`, return `queue_human` with the proposed reply.
5. If `mode` is `auto`, run `llm_guard`. Send only when approved.

## Pitfalls

- The guard is regex-based, not a real LLM. It catches obvious violations
  only.
- Medical/legal claim detection is heuristic; human review remains required
  for sensitive domains.
- PII regexes are simple; do not rely on them for compliance-grade redaction.
