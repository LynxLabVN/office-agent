# Lead-time tracker

| Gate | Started | Status | Blocks phase | ETA |
| ---- | ------- | ------ | ------------ | --- |
| Meta app review | | pending | Phase 3 (mcp-social-meta) | TBD |
| Zalo OA verification | | pending (start after OA business docs ready) | Phase 3 (mcp-zalo-oa) | TBD |
| TikTok Content Posting API | | pending | Phase 3 (mcp-social-tiktok) | TBD |
| YouTube Data API OAuth consent | | pending | Phase 3 (mcp-social-youtube) | TBD |
| base pin (agent-core commit) | 2026-07-05 | done 88d1d6206 | — | n/a |

## social-media skill map

The existing built-in social-media skill is `skills/social-media/xurl/` — an
X/Twitter skill wrapping the official `xurl` CLI. It exposes post, reply,
quote, delete, search, timeline, mentions, likes, reposts, follows, DMs, and
media upload workflows, all gated by OAuth 2.0 PKCE credentials stored in
`~/.xurl`. It is **not** used by the marketing pipeline (which targets
YouTube/Meta/TikTok), but it demonstrates Hermes' existing social gateway
patterns and is available as an auxiliary channel.

## company source → MCP map

To fill in after company source scripts are placed in `agent-core/company-src/` and inventoried.
