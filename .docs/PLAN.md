# Marketing + HR Agent — Build Plan

## Base: `LynxLabVN/agent-core` (fork of NousResearch/hermes-agent)

- Python 82% / TypeScript 14%. MIT.
- MCP-native via FastMCP. Has cron, skills, plugins, subagents, memory loop,
  React 19 + Vite dashboard, gateway (Telegram/Discord/Slack/etc.), TUI.
- Docs: https://hermes-agent.nousresearch.com/docs/

## Hermes Footprint Ladder (from AGENTS.md)

New capability priority:

`extend existing → CLI+skill → service-gated tool → plugin → MCP server → core tool (last resort)`

Rule: third-party product integrations ship as **standalone plugin repos**,
not in-tree. Our marketing MCPs follow this — standalone, registered in
`optional-mcps/` or `~/.hermes/plugins/`.

## External integration order (cross-cutting rule)

When a feature needs to touch a third-party service, pick the **first tier
that works**, never skip to a lower one:

```
1. Official API (OAuth/token, documented, ToS-blessed)   ← always prefer
2. Own controlled surface (our careers page, RSS, our OA) ← no third party sees automation
3. Human-in-loop manual (generate text → one-click copy → human pastes)
4. Browser automation (Playwright/Selenium against login-walled UI) ← LAST RESORT, never load-bearing
```

**Rule:** tier 4 is never a load-bearing path. It exists as a fallback for
job-board posting (VN boards have no API) only. If tier 4 is the only option,
the feature ships as tier 3 (human paste) until a real API appears.

## Bot-detection & quota risk tier (per surface)

Of the 12 external-touching surfaces, risk is uneven. Plan mitigations per tier:

| Tier | Surfaces | Risk type | Mitigation (already in plan) |
| ---- | -------- | --------- | ---------------------------- |
| **A — clean (local only)** | `mcp-catalog`, `mcp-ledger`, `mcp-video-edit`, `mcp-cv-screen` | None — no third-party calls | n/a |
| **B — official API gates** | `mcp-social-youtube`, `mcp-social-meta`, `mcp-social-tiktok`, `mcp-zalo-oa`, `mcp-schedule` | Quota / app-review lead time / ToS policy window — NOT bot detection | Backoff + quota budgeter (Phase 5); start Meta + Zalo OA verification in Phase 0; multiple YT projects if scale needed |
| **C — bot-detection exposed** | `hermes-zalo-plugin` (zca-js, personal Zalo), VN job-board browser-automation fallback | Account lock (Zalo pattern-detects automation); Cloudflare/captcha/login-walls (boards) | C1: secondary Zalo account, rate-limit self-throttle, 1-1 low-volume only, `ZALO_ALLOW_DESTRUCTIVE=false`. C2: own careers page first, browser automation = last resort only. |

**Architectural consequence:** 10 of 12 surfaces are tier A/B (clean or
API-gated). The 2 tier-C surfaces are isolated behind existing documented
fallbacks and must stay on tier 1-3 of the integration-order rule above.

## Pipeline (3 human gates, real workflow)

```
PRODUCT_SELECT → FORMAT_PICK → SCRIPT_DRAFT → HOOK_ITERATE →
  MANAGER_REVIEW_GATE → SHOOT_BRIEF → [HUMAN SHOOTS] →
  FOOTAGE_INGEST → AUTO_EDIT → FINAL_REVIEW_GATE → PUBLISH →
  MONITOR → REPLY/QUEUE → ANALYZE → LEDGER → (loop)
```

Human steps cannot be removed: shoot (physical), manager review (existing
process), optional final-video OK.

## Architecture — what touches what

| Component             | Hermes surface                                      | Action                                            |
| --------------------- | --------------------------------------------------- | ------------------------------------------------- |
| 7 MCP servers         | `optional-mcps/` + standalone repos                 | **Write new** (the "writing additional MCP" work) |
| Workflow orchestration| `skills/marketing-pipeline/SKILL.md` + CLI commands | **New skill** drives state machine via MCP tools + cron |
| Scheduler             | existing `cron/`                                    | Use as-is, register marketing jobs                |
| Memory/ledger         | existing memory system + `mcp-ledger`               | Hybrid: prose memory + structured SQLite          |
| Subagents             | existing delegation                                 | Use for parallel platform posting, comment triage |
| Web UI                | `web/src/pages/` + `hermes_cli/web_server.py`        | **Extend** (additive — welcomed per AGENTS.md)   |
| Social media skill    | existing `skills/social-media/`                     | **Extend** with marketing-specific commands       |
| Gateway               | existing                                            | Deliver manager-review notifications to Telegram/Zalo |
| Core agent loop       | `run_agent.py`                                      | **No modification** (prompt caching sacred)       |
| `toolsets.py` / core tools | —                                              | **No modification** (footprint ladder)            |

## MCP servers to write (7)

| Server                  | Tools                                                                   | Auth                              | Priority |
| ----------------------- | ----------------------------------------------------------------------- | --------------------------------- | -------- |
| `mcp-catalog`           | list_products, get_product_specs, search_catalog                        | None (local SQLite/YAML)          | P0       |
| `mcp-video-edit`        | cut_by_shotlist, burn_captions, overlay_text, add_music, encode_916, concat, extract_audio | None (FFmpeg)         | P0       |
| `mcp-ledger`            | record_post, get_performance, query_what_worked, get_hooks_leaderboard  | None (SQLite)                     | P0       |
| `mcp-social-youtube`    | upload, list_comments, reply_comment, get_stats, get_video_analytics   | OAuth2 (google-api-python-client) | P1       |
| `mcp-social-meta`       | post_reel, list_comments, reply, get_insights (IG + FB)                | OAuth2 (Meta Business)            | P1 (app review = weeks, start early) |
| `mcp-social-tiktok`     | post_video, get_metrics, list_comments                                  | OAuth2 (Content Posting API)      | P1       |
| `mcp-trend-research`    | search_hooks, get_trending_audio, fetch_reference_videos               | TT Research API (gated) + YT Data API | P2    |

**Language decision (locked):** 10 MCP servers in **Rust** (`rmcp` SDK + `tokio`
+ `reqwest` + `rusqlite`), 1 in **Python/FastMCP** (`mcp-cv-screen` only — needs
Whisper + PyMuPDF + Pillow). `hermes-zalo-plugin` (existing Node) used as-is.
Rationale: single static binary + no runtime deps for the glue/REST/SQLite/FFmpeg
servers; Python kept only where the parsing stack has no Rust equivalent.
Each MCP: stdio transport, registered in `optional-mcps/<name>/manifest.toml`
with binary path, env vars, and scopes. Company's existing
auto-reply/post/analytics source → wrapped into these (optimize + MCP-wrap).

## Skill: `marketing-pipeline`

`skills/marketing/marketing-pipeline/SKILL.md` — procedural playbook the agent
loads on `/marketing-pipeline` or auto-trigger. Encodes:

- 10 video formats: Short-form, UGC, Product Demo, Testimonial, Unboxing,
  Storytelling, BTS, Live Stream, Comparison, Skits
- 9-section script template (Content 1/7 set):
  1. Overview (product, goal, audience, platform, duration, format)
  2. Shoot requirements (aspect ratio, res/fps, tone & mood)
  3. Setting (location, time, lighting, rationale)
  4. Props & wardrobe
  5. Timeline & Shot list (duration, purpose, dialogue, action, angle, B-roll, props, on-screen text)
  6. Scenes (hook → product close-up → feature highlight → operation → demo → before/after → user reaction → CTA)
  7. Text On Screen (lines, keywords, price/offer)
  8. Shoot notes (pitfalls, must-see details, backup shots, per-scene time, priority)
  9. Pre-shoot checklist (hook, problem, benefit, demo, CTA, setting/angle/props/timeline, "đọc kịch bản là triển khai")
- Product catalog awareness: MA5, A14, AD35, A8, GX200, P011, bao đàn, UHF, mic đeo tai
- State machine transitions, gate logic, hook-iteration loop (gen + trend refs)
- Format-picker reasoning (product → format mapping)
- Output: ready-to-shoot brief

## UI redesign — new dashboard pages (additive)

Stack: React 19 + Vite + Tailwind v4 + shadcn-style (existing).
FastAPI backend at `hermes_cli/web_server.py`.

| New page              | Purpose                                                        | New API endpoints                                      |
| --------------------- | -------------------------------------------------------------- | ------------------------------------------------------ |
| `PipelinePage`        | Kanban: content pieces × pipeline states                       | `/api/pipeline/items`, `/api/pipeline/move`            |
| `ScriptWorkspacePage` | 9-section template editor + hook generator side-panel w/ refs | `/api/script/save`, `/api/script/generate`, `/api/hooks/suggest` |
| `ReviewQueuePage`     | Manager approve/revise w/ inline comments; routes to Telegram  | `/api/review/pending`, `/api/review/decide`           |
| `EditPreviewPage`     | Upload raw footage → auto-cut preview (xterm-less)            | `/api/edit/ingest`, `/api/edit/render`, `/api/edit/preview` |
| `PublishConsolePage`  | Platform toggles, schedule, per-platform caption               | `/api/publish/schedule`, `/api/publish/now`           |
| `CommentInboxPage`    | AI-suggested replies, approve/send, platform filter           | `/api/comments/list`, `/api/comments/reply`           |
| `AnalyticsPage`       | Per-product × per-format × per-platform charts; hooks leaderboard | `/api/analytics/overview`, `/api/analytics/drilldown` |
| `CatalogPage`         | Product CRUD                                                   | `/api/catalog/*`                                       |

Existing pages (StatusPage, ConfigPage, EnvPage, ChatPage) stay.
New nav entries added to `App.tsx`. API client extended in `web/src/lib/api.ts`.

## Phased plan

### Phase 0 — Recon & setup (~2 days)

- You: paste doc exports (4 Google Docs), borrow company tool source
- Me: inspect `skills/social-media/` existing content, map company source → MCP wrap plan
- **Start Meta app review process NOW** (weeks lead time)

### Phase 1 — MCP servers P0 (~1 week)

- `mcp-catalog` — SKU DB (MA5/A14/AD35/A8/GX200/P011/bao đàn/UHF/mic đeo tai)
- `mcp-video-edit` — FFmpeg-driven, Whisper for captions
- `mcp-ledger` — SQLite performance ledger
- Register in `optional-mcps/`, test via `hermes mcp` client

### Phase 2 — Skill + workflow (~3 days)

- `skills/marketing/marketing-pipeline/SKILL.md`
- CLI commands: `hermes marketing new <product>`, `hermes marketing review-queue`, `hermes marketing publish <id>`
- Cron jobs: scheduled posting, nightly analytics pull, weekly performance report
- Manager-review gate → Telegram notification (existing gateway)

### Phase 3 — MCP servers P1 (~1-2 weeks, gated by OAuth/app review)

- `mcp-social-youtube` — google-api-python-client, quota-aware
- `mcp-social-meta` — IG + FB, app review dependency
- `mcp-social-tiktok` — Content Posting API
- Wrap company source where applicable

### Phase 4 — UI redesign (~1-2 weeks)

- New pages in `web/src/pages/`
- New API endpoints in `hermes_cli/web_server.py`
- Build: `cd web && npm run build` → `hermes_cli/web_dist/`

### Phase 5 — MCP P2 + hardening (~1 week)

- `mcp-trend-research` (TT Research API gated — may need approval)
- Quota budgeter, retry/backoff, resumable uploads
- ToS-safe reply policy engine (allowlist + LLM guard)
- Audit log
- Auto-edit quality fallback: low-score → human editor path (full-auto first, fallback second)

## Risks

1. **TikTok comment reply** — no official endpoint. Deferred until tool arrives.
   Note: platform ToS ≠ Vietnamese law; ban risk is platform-side regardless.
2. **YouTube quota** — 10k units/day default, insert=1600 → ~6 uploads/day/project.
   Use multiple projects if scale needed. Unverified apps force `private` — audit for public.
3. **Meta app review** — weeks. Start Phase 0.
4. **Auto-edit quality ceiling** — skits/storytelling (formats 6,10) with 2 actors + dialogue
   hit ceiling. Fallback: quality-scorer → human editor path. Formats 1-5,7,9 (short/UGC/demo/
   testimonial/unboxing/BTS/comparison) = strong auto.
5. **Shoot brief clarity** — sample scripts already near "đọc kịch bản là triển khai" goal.
   Skill must match that quality.

## Verdict: High chance of "good"

- Hermes already has ~80% of infra (MCP, cron, plugins, skills, memory, gateway, dashboard, subagents)
- Work = extension (MCPs + skill + UI pages), not core surgery
- AGENTS.md explicitly welcomes dashboard/edge expansion
- Pipeline fits agent's strength (LLM gen + tool orchestration)
- Only true R&D risk = auto-edit quality for complex formats — mitigated by fallback

---

# HR Workflow Agent — Build Plan

## Scope (user-confirmed)

- **Domain:** Recruitment-focused (post jobs, CV/portfolio analysis, interview
  scheduling, candidate comms, dashboard UI)
- **Instance:** Single Hermes, dual-domain (marketing + HR coexist)
- **Data:** No existing system — build from scratch, local DB
- **Surfaces:** Job board posting, dashboard UI, interview scheduling, fast
  CV/portfolio screening on user command, messaging comms (Zalo/Telegram/Email)

## HR Pipeline (human gates)

```
JD_DRAFT → JD_REVIEW_GATE → POST_JOBS → RECEIVE_APPS →
  CV_SCREEN (on command) → SHORTLIST → SCHEDULE_INTERVIEW →
  INTERVIEW_NOTES → DECISION_GATE → OFFER → ONBOARD_HANDOFF
```

Human gates: JD review, interview decision/offer. Agent automates: JD draft,
multi-board posting, CV triage + scoring, scheduling, comms, reminders.

## HR Architecture — what touches what

| Component               | Hermes surface                                      | Action                                       |
| ----------------------- | --------------------------------------------------- | -------------------------------------------- |
| 4 MCP servers + 1 plugin | `optional-mcps/` + `cuongdev/hermes-zalo-plugin`    | **4 write new**, **1 use existing**          |
| HR workflow skill       | `skills/hr/recruitment/SKILL.md`                    | **New skill** — JD draft, CV scoring rubric, interview flow |
| Scheduler               | existing `cron/`                                    | Interview reminders, follow-up comms         |
| Memory / candidate DB   | `mcp-hr-data` (SQLite) + prose memory               | Candidate history, interview notes, decisions |
| Comms                   | existing gateway + `hermes-zalo-plugin` (installed) | Telegram/Email via existing gateway; Zalo via existing plugin (no MCP needed) |
| Web UI                  | `web/src/pages/` + `hermes_cli/web_server.py`        | **Extend** — new HR pages (additive)         |
| Core agent loop         | `run_agent.py`                                      | **No modification**                          |
| `toolsets.py`           | —                                                   | **No modification**                          |

## HR MCP servers to write (3) + 1 existing plugin

| Server / Plugin        | Tools                                                                         | Auth / Integration                | Priority |
| ---------------------- | ----------------------------------------------------------------------------- | --------------------------------- | -------- |
| `mcp-hr-data`          | create_job, list_jobs, get_job, save_application, list_applications, get_candidate, save_interview_note, update_stage, get_pipeline_stats | None (local SQLite) | P0 |
| `mcp-cv-screen`        | parse_cv, extract_skills, score_cv_against_jd, analyze_portfolio, compare_candidates, summarize_profile | None (LLM-driven via agent; PDF/image parsing via PyMuPDF+Pillow+Whisper for video portfolios) | P0 |
| `mcp-schedule`         | create_event_type, list_slots, book_slot, list_bookings, cancel_booking, send_invite | Cal.com API (OAuth or API key, 120 req/min) | P1 |
| `hermes-zalo-plugin`   | **USE EXISTING** — send, send-attachment, send-voice, react, typing, /api/<method> (145-method parity), contacts, groups, friends, polls | zca-js (unofficial, personal Zalo). QR login. SSE inbound + REST outbound. | P0 (install) |
| `mcp-zalo-oa`          | send_oa_message, send_oa_attachment, list_followers, get_user_profile, broadcast, query_message, tag_user, get_oa_profile, set_webhook | Zalo Official Account API v2.0 (official, business-verified). OAuth2 long-lived token + signed webhook inbound. | P1 (gated by OA verification) |

### Zalo comms via `hermes-zalo-plugin` (existing — no MCP needed)

Repo: `github.com/cuongdev/hermes-zalo-plugin` · npm: `hermes-zalo-plugin` · MIT.
Production-ready Node bridge: zca-js ↔ Hermes gateway adapter. 145-method Zalo API parity.

**Install (one-time):**
```bash
npm install -g hermes-zalo-plugin
hermes-zalo-plugin setup      # QR login (scan once) + background service
hermes gateway setup          # choose "Zalo" — wizard fetches contacts, pick users/groups
hermes gateway                # start relaying
```
Auto-installs adapter into `~/.hermes/plugins/zalo/`, enables `zalo-platform` in config.

**What it gives the HR agent for free:**
- Send candidate comms (text, attachment=CV/offer letter, voice, stickers, reactions)
- `POST /send` `{threadId, threadType, text, mentions, quote}` — DM + group
- `POST /send-attachment` `{threadId, threadType, path, caption}` — attach PDFs
- `POST /send-voice` — voice memo interview reminders
- `GET /contacts` `{groups:[{id,name}], friends:[{id,name}]}` — candidate lookup
- `GET /find-user?phone=` — look up candidate by phone
- Inbound `message` SSE events (incl. `mentions`, `quotedOwnerId`, `media`) → agent receives candidate replies
- `/api/<method>` passthrough to all 145 zca-js methods (subject to action-permission policy)
- Rate-limit self-throttling on info calls (anti account-lock)
- Access control: `ZALO_ALLOWED_USERS`, `ZALO_ALLOWED_THREADS`, `ZALO_GROUP_MODE` (mention/all/off)

**HR agent uses it via existing gateway** — no MCP server to write. Agent calls
`messages_send` (existing Hermes tool) with `target="zalo:<threadId>"`, or
directly via the bridge REST routes when needing media/features beyond text.

**Config for HR:**
- `ZALO_ALLOWED_USERS` = recruiter uids (or empty = allow all candidates — set allowlist)
- `ZALO_ALLOWED_THREADS` = candidate DM threads + recruitment group ids
- `ZALO_GROUP_MODE=mention` for shared recruitment groups
- `ZALO_ALLOWED_ACTION_GROUPS=read,send,interact` (no destructive ops)
- `ZALO_ALLOW_DESTRUCTIVE=false` (never for HR)

**Caveat:** zca-js is UNOFFICIAL — use a secondary Zalo account. Zalo may
rate-limit/lock accounts that automate. Accept risk or use Zalo OA API (official,
business-verified, lead time days) for high-volume outbound. For HR (low-volume, 1-1 candidate
comms), personal-account bridge is adequate.

**How it works:** zca-js reverse-engineers the personal Zalo **mobile app's own API**
(same endpoints the app uses for messages/media/groups/contacts). The plugin calls
those over HTTPS + WebSocket — hence 145-method parity. It is NOT a separate official
developer API; it's the app's internal protocol. Unofficial = not blessed by Zalo, may
break when they change the protocol (zca-js is actively maintained to keep up).
Account-lock risk = Zalo detecting automation patterns, not an API auth failure.

### Zalo OA (business account) — auto-reply & chat option

Personal bridge (`hermes-zalo-plugin` / zca-js) covers recruiter 1-1 DMs but
carries account-lock risk and cannot serve a **public business presence** (no
followers, no broadcast, no auto-reply on a verified brand identity). For
candidate-facing comms where people message the company page, add the official
path in parallel.

**Two-mode Zalo strategy:**

| Mode | Surface | Use for | Lock risk | Lead time |
| ---- | ------- | ------- | --------- | --------- |
| Personal (`hermes-zalo-plugin`) | DM + groups, 145 methods | Recruiter 1-1 chat, shared recruitment group | Yes (unofficial zca-js) | 30 min (QR) |
| Business OA (`mcp-zalo-oa`) | Followers, broadcast, webhook auto-reply | Public candidate comms via company OA, inbound auto-reply, official offer-letter delivery | None (official) | Days (verification) |

**`mcp-zalo-oa` — new MCP server (P1, gated by OA verification)**

Official Account API v2.0 · base `https://openapi.zalo.me/v2.0/officialaccount/`
· OAuth2 long-lived `access_token` (no QR, no session to keep alive) · inbound
via signed webhook callback (HTTP POST) — no SSE scraping.

Tools:
- `send_oa_message(userId, text|template)` — push to a follower (within 24h
  customer-service window since their last inbound)
- `send_oa_attachment(userId, type, upload_token|url, caption)` — image/file
  (offer letter PDF, JD attachment)
- `list_followers(offset, count)` — sync candidate list (those who followed OA)
- `get_user_profile(userId)` — name, avatar, gender (PII: audit-log access)
- `broadcast(message, segment)` — mass message (per-day quota; use for new-job
  announcements, NOT individual hiring decisions)
- `query_message(messageId)` — delivery/read status
- `tag_user(userId, tag)` — tag candidates (`interviewed`, `offered`, `rejected`)
- `get_oa_profile` — own OA info
- `set_webhook(url)` — register inbound endpoint (user replies, follow/unfollow,
  user-info-change events)
- Inbound webhook events → `comms_log` via `mcp-hr-data` hook (same sink as
  personal-bridge SSE path; `channel` field distinguishes `zalo_personal` vs
  `zalo_oa`)

**Auth:** `OA_ACCESS_TOKEN` + `OA_REFRESH_TOKEN` in Hermes env vault, rotate on
schedule. No personal QR, no account session to keep alive.

**Auto-reply & chat flow (the user-facing feature):**

1. Candidate messages company OA (sends CV, asks "còn tuyển không?").
2. Zalo fires webhook → `mcp-zalo-oa` receives `{event:message, userId, msg}`.
3. Gateway hook invokes agent loop (existing `run_agent.py`, no core mod) → LLM
   drafts reply gated by HR skill reply policy (allowlist templates + LLM guard,
   ToS-safe — same engine as social reply policy Phase 5).
4. Within 24h window: `send_oa_message` delivers. Outside window → queue human +
   nudge candidate to re-initiate (Zalo ToS: no push past 24h silence).
5. Every exchange logged to `comms_log` for audit + analytics.

**Config for HR (OA mode):**
- `ZALO_OA_TOKEN` = long-lived access token (env vault)
- `ZALO_OA_WEBHOOK_SECRET` = HMAC-verify inbound
- `ZALO_OA_REPLY_MODE` = `auto` (draft+send) | `suggest` (human approve) | `off`
  — default `suggest` for HR (PII + hiring sensitivity)
- `ZALO_OA_24H_POLICY` = enforce 24h customer-service window (reject push
  outside, queue instead)
- `ZALO_OA_BROADCAST_DAILY_CAP` = respect Zalo per-day broadcast quota

**Limitations vs personal bridge:**
- Only followers receive push → candidate must follow OA first (JD page / job
  post carries "Follow OA để nhận cập nhật" CTA).
- 24h customer-service window on 1-1 push (industry-standard, same as
  Messenger). Broadcast has no window but capped daily.
- No group access (OA can't read/send in user groups). Use personal bridge for
  shared recruitment groups.
- Verification + business docs required (days lead time — start HR Phase 0).

**Recommendation:** run BOTH. Personal bridge for recruiter's own 1-1 DM +
recruitment group; `mcp-zalo-oa` for company brand OA public auto-reply +
official offer-letter delivery. `mcp-hr-data` unifies both channels in
`comms_log` keyed by `channel`. HR skill picks channel per action:
candidate-initiated inbound on OA → OA reply; recruiter-initiated outbound DM →
personal bridge.

### Job board posting strategy (no open APIs for VN boards)

- **LinkedIn Jobs API** = gated (Talent Solutions partner only). Manual posting or partner status required.
- **VietnamWorks / TopCV / CareerBuilder VN** = no open public APIs.
- **Strategy:** `mcp-hr-data` exposes `post_to_board` tool that:
  - Posts to **own careers page** (auto-generated from `mcp-hr-data` jobs, served by dashboard) — always works
  - For external boards: agent generates board-formatted JD text + tags, human reviews, one-click copy or browser automation (Selenium/Playwright) for boards without APIs
  - Optional: RSS feed of open jobs for aggregator pickup
- Revisit when VN boards open APIs or partner access obtained.

### CV/portfolio screening (`mcp-cv-screen`) — "fast-checking on command"

Agent-invoked tools:
- `parse_cv(file_path)` → text + structured fields (name, contact, exp, skills, edu)
- `extract_skills(text)` → normalized skill list (map to JD keywords)
- `score_cv_against_jd(cv_id, jd_id)` → 0-100 score + breakdown (skills match, exp match, edu, gaps)
- `analyze_portfolio(url_or_files)` → screenshots/text extraction for web/GitHub; transcript for video portfolios (Whisper)
- `compare_candidates([cv_ids], jd_id)` → ranked table
- `summarize_profile(cv_id)` → 3-line human-readable summary
- Output: shortlist recommendation w/ rationale, human reviews before outreach

## HR Skill: `recruitment`

`skills/hr/recruitment/SKILL.md` — procedural playbook. Encodes:
- JD template (title, must-have/nice-to-have skills, exp level, salary range, location, benefits)
- CV scoring rubric (weighted: skills 40%, relevant exp 30%, portfolio quality 20%, education 10%)
- Interview flow (screen → technical → culture fit → offer)
- Question bank generator per role/skill
- Stage transition rules + auto-comms templates (acknowledge, interview invite, reject, offer)
- Vietnam labor law basics (probation max 60 days, contract types, SI/HC/HI notes) — for offer drafting

## HR UI — new dashboard pages (additive, dual-domain nav)

| New page              | Purpose                                                        | New API endpoints                                      |
| --------------------- | -------------------------------------------------------------- | ------------------------------------------------------ |
| `HRJobsPage`          | Job CRUD, JD editor, post-to-boards console                   | `/api/hr/jobs`, `/api/hr/jobs/<id>`, `/api/hr/post`    |
| `HRPipelinePage`      | Candidates kanban per job (Applied → Screened → Interview → Offer → Hired/Rejected) | `/api/hr/pipeline`, `/api/hr/pipeline/move` |
| `HRCandidatesPage`    | Candidate list, CV viewer, score breakdown, compare view      | `/api/hr/candidates`, `/api/hr/candidates/<id>`, `/api/hr/compare` |
| `HRSchedulePage`      | Interview slots, calendar sync (Cal.com embed), send invites  | `/api/hr/schedule/slots`, `/api/hr/schedule/book`      |
| `HRCommsPage`         | Candidate message log, Zalo/Telegram/Email threads, templates | `/api/hr/comms/log`, `/api/hr/comms/send`              |
| `HRAnalyticsPage`     | Time-to-hire, funnel conversion, source effectiveness         | `/api/hr/analytics/overview`                           |

Dashboard nav: top-level switch between **Marketing** and **HR** domains
(simple tab or route prefix `/hr/*`). Reuses existing shell, components, tokens.

## HR Phased plan

### HR Phase 0 — Recon & schema (~1 day)

- Define SQLite schema (jobs, candidates, applications, interviews, notes, comms_log with `channel` field for `zalo_personal` / `zalo_oa` / `telegram` / `email`)
- Choose: Cal.com (recommended, has API) vs self-hosted scheduling
- **Install `hermes-zalo-plugin`** + QR login + `hermes gateway setup` (Zalo) — one-time, ~30 min
- **Start Zalo OA verification** (business docs, days lead time) — register OA, prepare access token + webhook URL

### HR Phase 1 — MCP servers P0 + Zalo (~3-4 days)

- `mcp-hr-data` — full CRUD + pipeline stage machine
- `mcp-cv-screen` — PDF/image parse (PyMuPDF+Pillow), skill extraction, scoring,
  portfolio analysis (web scrape + Whisper for video)
- Verify Zalo plugin end-to-end: send test msg to candidate thread, receive reply
- Register MCPs in `optional-mcps/`, test

### HR Phase 2 — Skill + workflow (~2 days)

- `skills/hr/recruitment/SKILL.md`
- CLI commands: `hermes hr new-job`, `hermes hr screen <job>`, `hermes hr schedule <candidate>`
- Cron: interview reminders (candidate + interviewer), follow-up nudge if no reply in 48h

### HR Phase 3 — MCP server P1 (~2-3 days)

- `mcp-schedule` — Cal.com integration (event types, slots, bookings, cancel)
- `mcp-zalo-oa` — Official Account API: send_oa_message, broadcast, webhook
  auto-reply (`ZALO_OA_REPLY_MODE=suggest` default), tag candidates. Requires
  OA verification done in Phase 0.
- Inbound candidate replies flow from BOTH Zalo paths — personal bridge SSE +
  OA webhook → gateway → agent; log to `comms_log` keyed by `channel` via
  `mcp-hr-data` hook. HR skill routes: OA inbound → OA reply (within 24h
  window); recruiter outbound DM → personal bridge.
- Auto-reply policy engine (allowlist templates + LLM guard, ToS-safe — same
  engine as social reply policy Phase 5) wired to OA message handler
- Email comms via existing Hermes gateway (no new MCP)

### HR Phase 4 — UI (~4-5 days)

- 6 new HR pages in `web/src/pages/hr/`
- Nav domain switcher
- New API endpoints in `hermes_cli/web_server.py` under `/api/hr/*`
- Build → `hermes_cli/web_dist/`

### HR Phase 5 — Hardening (~2 days)

- CV scoring calibration (sample 20 CVs, tune weights)
- Audit log (who viewed/scored/moved candidate)
- PII handling: encrypt CV files at rest, access control on candidate data
- VN labor law compliance check on offer drafts

## HR Risks

1. **VN job boards no API** — mitigated by own careers page + manual/browser-automation fallback. Revisit if partner access obtained.
2. **LinkedIn Jobs API gated** — same mitigation.
3. **CV scoring accuracy** — LLM scoring needs calibration + human override always available. Start conservative, tune weights from real decisions.
4. **Zalo account risk** — `hermes-zalo-plugin` uses zca-js (unofficial personal-account API). Zalo may rate-limit/lock automated accounts. Mitigation: use secondary account, plugin has built-in rate-limit self-throttling (`ZALO_INFO_MIN_INTERVAL_MS`, backoff). For high-volume outbound or public auto-reply, use `mcp-zalo-oa` (official Zalo OA API, business-verified, no lock risk, days lead time). Run both: personal bridge for recruiter 1-1 DM/groups, OA for company brand public chat.
5. **Zalo OA 24h customer-service window** — official API forbids pushing 1-1 messages past 24h since candidate's last inbound. Mitigation: `ZALO_OA_24H_POLICY=enforce` rejects out-of-window push, queues human + nudges candidate to re-initiate. Broadcast (no window, daily cap) for job announcements only.
6. **PII/legal** — candidate data = sensitive. Encrypt at rest, audit log, restrict access. Vietnam Decree 13 (PDPL) compliance for data processing.

## Combined verdict

- Marketing + HR share same Hermes infra → economies of scope
- Both domains = extension work (MCPs + skills + UI), no core surgery
- HR has no existing system → clean greenfield, less integration debt than marketing
- Main HR risk = external API gaps (job boards) — mitigated by own careers page
- CV screening = strong agent use case (LLM + parsing + scoring)