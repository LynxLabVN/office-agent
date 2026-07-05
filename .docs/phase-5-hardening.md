# Phase 5 — P2 + hardening

**Goal:** `mcp-trend-research` (P2), quota budgeter, retry/backoff, audit log,
auto-edit quality fallback, CV scoring calibration, PII encryption.
**Depends on:** Phase 1 + 1b + 3.
**Duration:** ~1 week.
**Parallelizable with:** Phase 4.

Read [`00-overview.md`](./00-overview.md) first for workspace structure and
conventions.

---

## 5.1 `mcp-trend-research` (Rust + reqwest)

**Files:**
- `optional-mcps/mcp-trend-research/src/{main,tools,client}.rs`
- `optional-mcps/mcp-trend-research/manifest.toml`

**Auth:** TikTok Research API (gated — may need approval) + YouTube Data API
(search). Tokens: `TT_RESEARCH_CLIENT_KEY/SECRET`, reuses `YOUTUBE_*`.

**Tools:**

| Tool | Params | Returns | API |
| ---- | ------ | ------- | --- |
| `search_hooks` | `product_category, region?: "VN"` | `[{hook_text, platform, views, likes, created_at}]` | TT Research + YT search |
| `get_trending_audio` | `region?: "VN"` | `[{audio_id, title, platform, uses_count}]` | TT trending sounds |
| `fetch_reference_videos` | `query, platform, limit?` | `[{video_id, url, title, views}]` | YT search + TT search |
| `health` | — | `{"ok":true}` | — |

**Gate:** if TT Research API not approved, `search_hooks` + `get_trending_audio`
return error `"TT Research API pending — see LEADTIMES.md"`. Skill handles
gracefully (falls back to LLM-generated hooks from `mcp-ledger` history).

**Verify:**
- `cargo test -p mcp-trend-research` — mock HTTP.
- If TT approved: `hermes mcp call mcp-trend-research search_hooks '{"product_category":"audio"}'`
  → returns hooks.
- If not approved: returns graceful error, skill doesn't crash.

## 5.2 Quota budgeter (shared)

**File:** `agent-core/skills/quota-budgeter/` (shared module).

**Do:**
- Centralized quota tracking for YouTube (10000 units/day, insert=1600),
  Meta (rate headers), TikTok (rate headers), Cal.com (120/min).
- `~/.hermes/data/quota_state.json` — per-platform counters with reset times.
- Before any upload/post call, check budget. If insufficient → return
  `retry_after` + queue for next reset.
- Implement exponential backoff with jitter on 429/503.

**Verify:**
- Unit test: simulate 7 YouTube uploads (7×1600=11200 > 10000) → 7th is
  rejected with `retry_after`.
- Unit test: backoff sequence on 429 → 1s, 2s, 4s, 8s (with jitter bounds).

## 5.3 Resumable uploads

**Do:** YouTube + Meta uploads use resumable protocol. On network failure,
resume from last byte. Track upload state in `~/.hermes/data/uploads/`.

**Verify:**
- Unit test: mock upload that fails at 50% → resume continues from 50%.

## 5.4 Audit log

**File:** `agent-core/skills/audit/` (shared module) + `mcp-ledger` +
`mcp-hr-data` hooks.

**Do:**
- Every state transition (pipeline + HR), every MCP tool call, every
  auto-reply, every human decision → append to `~/.hermes/data/audit.log`
  (JSONL).
- Fields: `ts, actor (agent|human:<name>), action, target, before, after`.
- API endpoint: `/api/audit?from=&to=&actor=&action=` for dashboard.

**Verify:**
- Run a full marketing pipeline cycle → `audit.log` has entries for each
  state transition + each MCP call.
- `curl localhost:8000/api/audit?from=2026-07-01` → returns JSONL.

## 5.5 Auto-edit quality fallback

**File:** `agent-core/skills/marketing/quality-scorer.py` (new module).

**Do:**
- After `mcp-video-edit` produces a render, score it:
  - Hook retention prediction (first 3s has motion + text + clear subject?)
  - Audio quality (FFmpeg loudnorm check, no clipping)
  - Caption accuracy (Whisper confidence > 0.8)
  - Resolution/fps correct (ffprobe)
- Score 0-100. If < 60 → flag for human editor path (queue in
  `ReviewQueuePage` with "auto-edit low quality" tag).
- Formats 1-5,7,9 (short/UGC/demo/testimonial/unboxing/BTS/comparison) =
  strong auto. Formats 6,10 (storytelling/skits) = always human review.

**Verify:**
- Feed a good render → score > 70, auto-approved.
- Feed a render with bad audio (clipping) → score < 60, flagged.
- Skit format → always flagged regardless of score.

## 5.6 CV scoring calibration

**Do:**
- Collect 20 sample CVs (user-provided) + 5 sample JDs.
- Run `mcp-cv-screen.score_cv_against_jd` on all.
- Human reviews scores, notes disagreements.
- Tune weights in `skills/hr/recruitment/SKILL.md` rubric section:
  - If skills weight too high → reduce 40% → 35%, bump exp 30% → 35%.
  - Document final weights + rationale in SKILL.md.
- Add `score_override` field to `applications` table (human can override
  agent score; original kept in `score_breakdown`).

**Verify:**
- 20 CVs scored, human-reviewed, weights tuned.
- `hermes mcp call mcp-hr-data` — `score_override` field works.

## 5.7 PII handling + encryption

**Do:**
- CV files stored at `~/.hermes/data/cv/` encrypted at rest (age or
  `rusqlite` SQLCipher).
- Access control: only `recruiter` role users can view candidate data via
  dashboard. Add `role` field to Hermes user config.
- `get_user_profile` (Zalo OA) calls audit-logged.
- Vietnam Decree 13 (PDPL) compliance: data retention policy (delete
  candidate data 12 months after rejection unless consent), document in
  `skills/hr/recruitment/SKILL.md`.

**Verify:**
- CV files at rest are encrypted (`file ~/.hermes/data/cv/*.pdf` shows
  encrypted, not PDF header).
- Non-recruiter user → `/api/hr/candidates` returns 403.
- `audit.log` has entry when `get_user_profile` is called.

## 5.8 VN labor law compliance check on offers

**Do:**
- In `skills/hr/recruitment/SKILL.md` offer-drafting section, add a checklist:
  - Probation ≤ 60 days (Labor Code Art 27)
  - Contract type specified (indefinite/definite/seasonal)
  - SI/HC/HI employer contribution noted (17.5% salary + 8% from employee)
  - Working hours ≤ 8h/day, 48h/week
  - Overtime caps + premium rates (1.5x weekday, 2x weekend, 3x holiday)
- Agent drafts offer → checklist auto-verified → if any item missing, flag
  for human review.

**Verify:**
- Draft an offer missing probation clause → agent flags it.
- Draft a complete offer → passes checklist.

---

## Phase 5 exit criteria

- [ ] `mcp-trend-research`: build + test passes (graceful error if API pending)
- [ ] Quota budgeter: unit tests pass, blocks 7th YouTube upload
- [ ] Audit log: full pipeline cycle produces complete JSONL trail
- [ ] Quality scorer: good render passes, bad render flagged, skits always flagged
- [ ] 20 CVs scored + calibrated, weights documented
- [ ] CV files encrypted at rest, role-based access works
- [ ] Offer checklist catches missing probation clause
