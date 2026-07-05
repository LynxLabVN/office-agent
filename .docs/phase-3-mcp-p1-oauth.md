# Phase 3 — MCP P1 (OAuth-gated)

**Goal:** 5 MCP servers (3 social + schedule + zalo-oa) working with real
OAuth tokens. Auto-reply policy engine wired.
**Depends on:** Phase 0 app reviews complete (Meta, Zalo OA, TikTok, YouTube)
+ Phase 1 workspace patterns.
**Duration:** ~1.5-2 weeks.
**Parallelizable with:** Phase 2 (skills can be drafted against stubs).

Read [`00-overview.md`](./00-overview.md) first for workspace structure and
conventions.

**Critical:** Do NOT start a sub-phase if its app review is not complete.
Check `LEADTIMES.md` first. If a review is still pending, build the crate +
unit tests with mocked HTTP, but don't do integration verification.

---

## 3.1 `mcp-social-youtube` (Rust + reqwest + OAuth2)

**Files:**
- `optional-mcps/mcp-social-youtube/src/{main,tools,client,oauth}.rs`
- `optional-mcps/mcp-social-youtube/manifest.toml`

**OAuth flow:**
- Desktop app OAuth2 (Google): `https://accounts.google.com/o/oauth2/v2/auth`
  → callback to `http://localhost:<port>/callback` → exchange code for
  `access_token` + `refresh_token`.
- Tokens stored in Hermes env vault: `YOUTUBE_CLIENT_ID`,
  `YOUTUBE_CLIENT_SECRET`, `YOUTUBE_REFRESH_TOKEN`.
- Auto-refresh on 401 using `refresh_token`.

**Tools:**

| Tool | Params | Returns | API |
| ---- | ------ | ------- | --- |
| `upload` | `video_path, title, description, tags[], category_id?, privacy?` | `{video_id, url}` | POST `youtube.upload.v3` (resumable, 1600 quota units) |
| `list_comments` | `video_id, max_results?` | `[{id, author, text, published_at}]` | GET `commentThreads.list` |
| `reply_comment` | `comment_id, text` | `{reply_id}` | POST `comments.insert` |
| `get_stats` | `video_id` | `{views, likes, comments, shares}` | GET `videos.list` part=statistics |
| `get_video_analytics` | `video_id, start_date, end_date` | `{views, watch_time, avg_view_duration, impressions, ctr}` | GET YouTube Analytics API |
| `health` | — | `{"ok":true}` | — |

**Quota handling:** `upload` checks a local quota counter (`~/.hermes/data/yt_quota.json`) before each call. Default 10000 units/day, `upload` = 1600. If remaining < 1600, return error with `retry_after` timestamp. Reset counter at 00:00 Pacific.

**Verify (only if YouTube OAuth done):**
- `cargo test -p mcp-social-youtube` — mock HTTP, assert request construction.
- `hermes mcp call mcp-social-youtube upload '{"video_path":"tests/fixtures/test.mp4","title":"Test","privacy":"private"}'`
  → returns `{video_id}`. (Use `private` to avoid public test videos.)
- `hermes mcp call mcp-social-youtube get_stats '{"video_id":"<id>"}'` →
  returns stats.

## 3.2 `mcp-social-meta` (Rust + reqwest + OAuth2)

**Files:**
- `optional-mcps/mcp-social-meta/src/{main,tools,client,oauth}.rs`
- `optional-mcps/mcp-social-meta/manifest.toml`

**OAuth:** Meta Business — Page access token + Instagram Business account
linked. Tokens: `META_APP_ID`, `META_APP_SECRET`, `META_PAGE_ACCESS_TOKEN`,
`META_IG_USER_ID`.

**Tools:**

| Tool | Params | Returns | API |
| ---- | ------ | ------- | --- |
| `post_reel` | `video_path, caption, platform: "instagram"\|"facebook", thumb?` | `{media_id, permalink}` | IG: 2-step (create container + publish). FB: `/page/videos` |
| `list_comments` | `media_id, platform` | `[{id, author, text, timestamp}]` | IG: `/media/comments`. FB: `/{post-id}/comments` |
| `reply` | `comment_id, platform, text` | `{reply_id}` | `/{comment-id}/replies` |
| `get_insights` | `media_id, platform, metrics[]` | `{metric: value}` | IG: `/media/insights`. FB: `/{post-id}/insights` |
| `health` | — | `{"ok":true}` | — |

**Verify (only if Meta app review done):**
- `cargo test -p mcp-social-meta` — mock HTTP.
- `hermes mcp call mcp-social-meta post_reel '{"video_path":"...","caption":"test","platform":"instagram"}'`
  → returns `{media_id}`.

## 3.3 `mcp-social-tiktok` (Rust + reqwest + OAuth2)

**Files:**
- `optional-mcps/mcp-social-tiktok/src/{main,tools,client,oauth}.rs`
- `optional-mcps/mcp-social-tiktok/manifest.toml`

**OAuth:** Content Posting API — TikTok Login Kit + Video Upload scope.
Tokens: `TIKTOK_CLIENT_KEY`, `TIKTOK_CLIENT_SECRET`, `TIKTOK_ACCESS_TOKEN`.

**Tools:**

| Tool | Params | Returns | API |
| ---- | ------ | ------- | --- |
| `post_video` | `video_path, title, privacy_level, tags[]?` | `{video_id, share_url}` | init upload + chunk upload + post |
| `get_metrics` | `video_id` | `{views, likes, comments, shares, watch_time}` | `/video/query/` |
| `list_comments` | `video_id, max_results?` | `[{id, author, text, created_at}]` | `/comment/list/` |
| `health` | — | `{"ok":true}` | — |

**Note:** TikTok has NO comment reply endpoint (PLAN.md Risk #1). `list_comments`
exists but reply is deferred. Do NOT implement a reply tool.

**Verify (only if TikTok API access granted):**
- `cargo test -p mcp-social-tiktok` — mock HTTP.
- `hermes mcp call mcp-social-tiktok post_video '{"video_path":"...","title":"test","privacy_level":"MUTUAL_FOLLOW_NETWORK"}'`
  → returns `{video_id}`.

## 3.4 `mcp-schedule` (Rust + reqwest + Cal.com API)

**Files:**
- `optional-mcps/mcp-schedule/src/{main,tools,client}.rs`
- `optional-mcps/mcp-schedule/manifest.toml`

**Auth:** Cal.com API key. `$CALCOM_API_KEY`, `$CALCOM_API_URL` (default
`https://api.cal.com/v1`).

**Tools:**

| Tool | Params | Returns | API |
| ---- | ------ | ------- | --- |
| `create_event_type` | `title, length_min, slug` | `{event_type_id}` | POST `/event-types` |
| `list_slots` | `event_type_id, date_from, date_to` | `[{start, end, available}]` | GET `/slots` |
| `book_slot` | `event_type_id, start, name, email, notes?` | `{booking_id, uid}` | POST `/bookings` |
| `list_bookings` | `status?: "upcoming"\|"past"\|"cancelled"` | `[{id, title, start, end, attendee_name, attendee_email}]` | GET `/bookings` |
| `cancel_booking` | `booking_id, reason?` | `{ok:true}` | DELETE `/bookings/{id}` |
| `send_invite` | `booking_id, channel: "email"\|"zalo"` | `{ok:true}` | sends via Hermes gateway (email) or zalo plugin |
| `health` | — | `{"ok":true}` | — |

**Rate limit:** Cal.com = 120 req/min. Implement a token-bucket limiter in
`client.rs`.

**Verify:**
- `cargo test -p mcp-schedule` — mock HTTP.
- `hermes mcp call mcp-schedule create_event_type '{"title":"Technical Interview","length_min":60,"slug":"tech-interview"}'`
  → returns `{event_type_id}`.
- `hermes mcp call mcp-schedule list_slots '{"event_type_id":"<id>","date_from":"2026-07-10","date_to":"2026-07-11"}'`
  → returns slots.
- `hermes mcp call mcp-schedule book_slot '{"event_type_id":"<id>","start":"2026-07-10T10:00:00Z","name":"Nguyen Van A","email":"a@b.com"}'`
  → returns `{booking_id}`.

## 3.5 `mcp-zalo-oa` (Rust + reqwest + Zalo OA API v2.0)

**Files:**
- `optional-mcps/mcp-zalo-oa/src/{main,tools,client,webhook}.rs`
- `optional-mcps/mcp-zalo-oa/manifest.toml`

**Auth:** `ZALO_OA_TOKEN` (long-lived), `ZALO_OA_WEBHOOK_SECRET` (HMAC).
Base URL: `https://openapi.zalo.me/v2.0/officialaccount/`.

**Tools:**

| Tool | Params | Returns | API |
| ---- | ------ | ------- | --- |
| `send_oa_message` | `user_id, text` or `template` | `{message_id}` | POST `/message` |
| `send_oa_attachment` | `user_id, type: "image"\|"file", url, caption?` | `{message_id}` | POST `/message/attachment` |
| `list_followers` | `offset?, count?` | `[{user_id, display_name}]` | GET `/list-followers` |
| `get_user_profile` | `user_id` | `{user_id, display_name, avatar, gender}` | GET `/profile` |
| `broadcast` | `message, segment?` | `{broadcast_id}` | POST `/broadcast/message` |
| `query_message` | `message_id` | `{status, sent_time}` | GET `/message/status` |
| `tag_user` | `user_id, tag` | `{ok:true}` | POST `/tag` |
| `get_oa_profile` | — | `{name, avatar, ...}` | GET `/profile` |
| `set_webhook` | `url` | `{ok:true}` | POST `/webhook` |
| `health` | — | `{"ok":true}` | — |

**Webhook inbound (in `webhook.rs`):**
- HTTP server (axum) on `$ZALO_OA_WEBHOOK_PORT` (default 8080).
- HMAC-verify `X-Zalo-Signature` header against `ZALO_OA_WEBHOOK_SECRET`.
- On `message` event → invoke Hermes agent loop (existing gateway hook, no
  core mod) → LLM drafts reply → gated by reply policy (3.6).
- Enforce 24h window: check `comms_log` for candidate's last inbound; if >24h,
  queue human + send nudge template instead of auto-reply.
- Log every exchange to `comms_log` via `mcp-hr-data` hook.

**Config (Hermes env):**
```
ZALO_OA_REPLY_MODE=suggest    # auto | suggest | off (default suggest for HR)
ZALO_OA_24H_POLICY=enforce
ZALO_OA_BROADCAST_DAILY_CAP=100000
```

**Verify (only if Zalo OA verified):**
- `cargo test -p mcp-zalo-oa` — mock HTTP + webhook.
- `hermes mcp call mcp-zalo-oa send_oa_message '{"user_id":"<follower_id>","text":"Xin chào"}'`
  → returns `{message_id}`.
- Send a message to the OA from a follower phone → webhook fires → agent
  drafts reply → with `ZALO_OA_REPLY_MODE=suggest`, reply queued for human
  approval (not auto-sent).
- `comms_log` has both inbound + queued outbound rows with `channel=zalo_oa`.

## 3.6 Auto-reply policy engine (shared — social + HR)

**File:** `agent-core/skills/reply-policy/` (a shared module, not an MCP).

**Purpose:** gate all outbound auto-replies (social comments + Zalo OA) through
a ToS-safe allowlist + LLM guard.

**Logic:**
1. **Allowlist templates:** approved reply templates per platform + scenario
   (e.g., "thank you for your question", "DM us for details", reject
   templates). Stored in `skills/reply-policy/templates.toml`.
2. **LLM guard:** if no template matches, agent drafts a reply → guard
   checks: no promises/commitments, no PII, no salary specifics (HR), no
   medical/legal claims, under 200 chars. If guard fails → queue human.
3. **Mode:** `auto` (draft + send), `suggest` (draft + queue human), `off`.
4. **Audit:** every auto-reply + decision logged to `comms_log` (HR) or
   `mcp-ledger` (social).

**Verify:**
- Unit test: template matcher returns correct template for 5 scenarios.
- Unit test: LLM guard rejects a reply containing "we guarantee" and a reply
  with a phone number.
- Integration: social comment reply in `suggest` mode → queued, not sent.

## 3.7 Email comms (no new MCP)

**Do:** Use existing Hermes gateway email support. Verify `messages_send` with
`target="email:<addr>"` works. No new code.

**Verify:** `hermes mcp call hermes messages_send '{"target":"email:test@example.com","text":"test"}'`
→ email arrives.

---

## Phase 3 exit criteria

- [ ] All 5 new MCP crates: `cargo build --workspace --release` succeeds
- [ ] All 5: `cargo test` passes (mocked HTTP if app review pending)
- [ ] For each app review that's done: integration `hermes mcp call` works
      against real API
- [ ] Zalo OA webhook fires on inbound → reply queued in `suggest` mode
- [ ] Reply policy engine: unit tests pass, guard rejects violations
- [ ] `hermes mcp list` shows all 11 MCPs (10 Rust + 1 Python) + existing
