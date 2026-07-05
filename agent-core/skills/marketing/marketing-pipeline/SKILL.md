---
name: marketing-pipeline
description: "Use when the user asks for a new marketing video, runs /marketing-pipeline, or mentions creating content for a LynxLabVN product. Drives the 14-state video pipeline from product selection through publish and analysis using the marketing MCP servers."
version: 1.0.0
author: Hermes Agent
license: MIT
platforms: [linux, macos, windows]
metadata:
  hermes:
    tags: [marketing, video, pipeline, content, lynxlab]
    related_skills: []
---

# Marketing Pipeline Skill

## Overview

This skill runs the LynxLabVN marketing video pipeline end-to-end. It turns a
product SKU (or a free-form request like "new video for MA5") into a ready-to-
shoot brief, optionally coordinates auto-edit after footage is ingested,
publishes to social platforms, and records performance back into the ledger.

The pipeline is a state machine with 14 states and 3 human gates. The agent must
pause at each gate and wait for a human decision before continuing.

## When to Use

- `/marketing-pipeline` slash command.
- User says "new video for `<product>`", "làm clip cho `<sku>`", "viết kịch bản
  MA5", etc.
- A cron job fires `hermes marketing publish-queue` or `hermes marketing
  pull-analytics`.
- The manager clicks **approve** or **revise** on a Telegram review message.

## Prerequisites

The following MCP servers must be configured in `~/.hermes/config.yaml` under
`mcp_servers:`:

- `mcp-catalog` — product database (MA5, A14, AD35, A8, GX200, P011, bao đàn,
  UHF, mic đeo tai).
- `mcp-ledger` — performance ledger and hooks leaderboard.
- `mcp-video-edit` — FFmpeg-based auto edit (cut, encode 9:16, burn captions,
  add music).
- `mcp-social-youtube` (Phase 3) — upload + comments + stats.
- `mcp-social-meta` (Phase 3) — IG/FB reel posting.
- `mcp-social-tiktok` (Phase 3) — TikTok posting.
- `mcp-trend-research` (Phase 5) — hook / audio / reference research.

Hermes gateway must be running for Telegram manager-review notifications and
for social comment replies.

## Pipeline State Machine (14 states)

```
PRODUCT_SELECT → FORMAT_PICK → SCRIPT_DRAFT → HOOK_ITERATE →
  MANAGER_REVIEW_GATE → SHOOT_BRIEF → [HUMAN SHOOTS] →
  FOOTAGE_INGEST → AUTO_EDIT → FINAL_REVIEW_GATE → PUBLISH →
  MONITOR → REPLY/QUEUE → ANALYZE → LEDGER → (loop)
```

Human gates (the agent must pause and wait):

1. `MANAGER_REVIEW_GATE` — script / hook / format approval.
2. `FINAL_REVIEW_GATE` — optional final video OK before publishing.
3. `SHOOT_BRIEF` output is executed by a human crew (camera, lighting, talent).
   The agent cannot replace this step.

### 1. PRODUCT_SELECT

- **Entry trigger:** `/marketing-pipeline` or "new video for `<sku>`".
- **Exit condition:** A valid SKU is resolved and a product record exists in
  `mcp-catalog`.
- **MCP call:** `mcp-catalog.list_products` (to show options if SKU is missing),
  then `mcp-catalog.get_product_specs` for the chosen SKU.
- **Next state:** `FORMAT_PICK`.

### 2. FORMAT_PICK

- **Entry condition:** Product specs are available.
- **Exit condition:** One or more recommended formats are chosen using the
  format picker table below.
- **MCP call:** None (agent LLM decision).
- **Next state:** `SCRIPT_DRAFT`.

### 3. SCRIPT_DRAFT

- **Entry condition:** Product + format are known.
- **Exit condition:** A complete 9-section script is generated.
- **MCP call:** `mcp-ledger.get_hooks_leaderboard` (historical hooks for this
  product category, used as inspiration, not copy-paste).
- **Next state:** `HOOK_ITERATE`.

### 4. HOOK_ITERATE

- **Entry condition:** Script draft exists.
- **Exit condition:** 3 hook variants generated, optionally trend-researched,
  scored, and the best hook selected.
- **MCP call:**
  - `mcp-trend-research.search_hooks` (Phase 5; gate if not ready).
  - `mcp-ledger.get_hooks_leaderboard` (score against historical performance).
- **Next state:** `MANAGER_REVIEW_GATE`.

### 5. MANAGER_REVIEW_GATE

- **Entry condition:** Script + best hook + format are ready.
- **Exit condition:** Manager approves or requests revision via Telegram
  callback.
- **MCP call / tool:** `messages_send` to Telegram target
  `telegram:<MANAGER_TELEGRAM_CHAT_ID>` with product name, format, hook, script
  summary, and inline approve/revise buttons.
- **Approve →** `SHOOT_BRIEF`.
- **Revise →** return to `SCRIPT_DRAFT` with manager feedback appended to the
  piece record.

### 6. SHOOT_BRIEF

- **Entry condition:** Script approved.
- **Exit condition:** A shoot-ready brief (equipment, location, shot list,
  schedule) is emitted and delivered to the crew.
- **MCP call:** None (agent formats the brief from the 9-section script).
- **Next state:** `FOOTAGE_INGEST` after human shoot completes.

### 7. FOOTAGE_INGEST

- **Entry condition:** Raw footage files are available on disk.
- **Exit condition:** Footage is cut by shotlist and ready for auto-edit.
- **MCP call:** `mcp-video-edit.cut_by_shotlist`.
- **Next state:** `AUTO_EDIT`.

### 8. AUTO_EDIT

- **Entry condition:** Cut clips exist.
- **Exit condition:** 9:16 encoded video with captions and music is rendered.
- **MCP call:**
  - `mcp-video-edit.encode_916`
  - `mcp-video-edit.burn_captions`
  - `mcp-video-edit.add_music`
- **Next state:** `FINAL_REVIEW_GATE`.

### 9. FINAL_REVIEW_GATE

- **Entry condition:** Final rendered video exists.
- **Exit condition:** Manager (or designated reviewer) gives final OK.
- **MCP call / tool:** `messages_send` with video file/ link and approve/revise
  buttons.
- **Approve →** `PUBLISH`.
- **Revise →** return to `AUTO_EDIT` with notes.

### 10. PUBLISH

- **Entry condition:** Final video approved.
- **Exit condition:** Video is uploaded to every enabled platform and recorded
  in the ledger.
- **MCP call:**
  - `mcp-social-youtube.upload` (Phase 3)
  - `mcp-social-meta.post_reel` (Phase 3)
  - `mcp-social-tiktok.post_video` (Phase 3)
  - `mcp-ledger.record_post`
- **Next state:** `MONITOR`.

### 11. MONITOR

- **Entry condition:** Posts are live.
- **Exit condition:** Stats for the last period are fetched.
- **MCP call:** `mcp-social-*.get_stats` per platform.
- **Next state:** `REPLY/QUEUE`.

### 12. REPLY/QUEUE

- **Entry condition:** Stats fetched.
- **Exit condition:** Comments are triaged; replies requiring human tone are
  queued for approval; safe replies are sent.
- **MCP call / tool:** `messages_send` for human-queued replies;
  `mcp-social-*.reply_comment` for approved safe replies.
- **Next state:** `ANALYZE`.

### 13. ANALYZE

- **Entry condition:** Performance data and comment sentiment are available.
- **Exit condition:** Summary of what worked is produced.
- **MCP call:**
  - `mcp-ledger.get_performance`
  - `mcp-ledger.query_what_worked`
- **Next state:** `LEDGER`.

### 14. LEDGER

- **Entry condition:** Analysis complete.
- **Exit condition:** Hook, format, and performance insights are written back
  to `mcp-ledger` for future runs.
- **MCP call:** `mcp-ledger.record_post` (update) or a custom ledger write.
- **Next state:** Pipeline ends; the next piece starts again at
  `PRODUCT_SELECT`.

## Format Picker Decision Table

Use this table to map the product category to the recommended format mix. The
agent should explain its reasoning in one sentence per recommendation.

| Product | Category | Recommended format(s) | Reasoning |
| --- | --- | --- | --- |
| MA5 | visual (smart glasses / AR device) | UGC + unboxing + BTS | Visual-first product; social proof and behind-the-scenes build trust and show the device in real use. |
| A14 | visual (action camera / drone accessory) | UGC + unboxing + BTS | Buyers want to see image quality and real-world mounting; UGC proves it. |
| AD35 | visual (display / monitor accessory) | UGC + unboxing + BTS | Screen products need unboxing to show panel, build, and ports; BTS adds production value. |
| A8 | visual (compact camera / gimbal) | UGC + unboxing + BTS | Demo-heavy; BTS shows stability and portability. |
| GX200 | visual (lighting / grip rig) | UGC + unboxing + BTS | Creators want to see the rig in a real setup; unboxing covers components. |
| P011 | visual (mobile rig / accessory) | UGC + unboxing + BTS | Pocket-sized visual accessory; UGC shows everyday carry use cases. |
| bao đàn | audio (guitar bag / case) | demo + comparison + testimonial | Function is protection and portability; comparison proves value; testimonial adds emotional trust. |
| UHF | audio (wireless microphone system) | demo + comparison + testimonial | Audio quality is invisible; demo + comparison make it audible; testimonial adds reliability. |
| mic đeo tai | audio (lavalier / headset mic) | demo + comparison + testimonial | Clarity and range matter; comparison against phone/built-in mic is persuasive. |
| *(new launch)* | any | short-form + storytelling | Launches need fast reach; short-form drives awareness and storytelling creates emotional hook. |

For audio products, add a "before/after" or "with vs without" segment inside the
demo so the audience can hear the difference.

## 9-Section Script Template

Every script must have these 9 sections. The agent fills every field. Empty
fields are not allowed.

### Section 1 — Overview

| Field | Type | Example |
| --- | --- | --- |
| `product` | string | MA5 |
| `goal` | string | Drive pre-orders among creator/tech audience |
| `audience` | string | Vietnamese tech creators, 18-35, Facebook + TikTok |
| `platform` | string | TikTok, Facebook Reels |
| `duration` | integer (seconds) | 45 |
| `format` | string | UGC-style unboxing |

### Section 2 — Shoot requirements

| Field | Type | Example |
| --- | --- | --- |
| `aspect_ratio` | string | 9:16 |
| `resolution_fps` | string | 1080p60 |
| `tone_mood` | string | Curious, energetic, premium-but-accessible |

### Section 3 — Setting

| Field | Type | Example |
| --- | --- | --- |
| `location` | string | Clean desk near window |
| `time` | string | Late morning, natural light + 1 softbox |
| `lighting` | string | Key light 45° camera left, fill from window |
| `rationale` | string | Shows product clearly without looking sterile |

### Section 4 — Props & wardrobe

| Field | Type | Example |
| --- | --- | --- |
| `props` | list[string] | MA5 unit, charging cable, phone, branded mat |
| `wardrobe` | string | Neutral solid-color shirt, no logos |

### Section 5 — Timeline & shot list

List of shots. Each shot:

| Field | Type | Example |
| --- | --- | --- |
| `duration` | integer (seconds) | 5 |
| `purpose` | string | Hook: grab attention |
| `dialogue` | string | "Mở hộp chiếc kính AR nhỏ gọn nhất từng thấy." |
| `action` | string | Hands pull box open, reveal MA5 |
| `angle` | string | Top-down |
| `b_roll` | string | Close-up of box texture |
| `props` | list[string] | Box, cutter |
| `on_screen_text` | string | "MA5 — kính AR made in VN" |

### Section 6 — Scenes

Ordered list of scenes. Recommended order:

1. Hook (0-3 s)
2. Product close-up / unboxing (3-10 s)
3. Feature highlight (10-20 s)
4. Operation / how it works (20-30 s)
5. Demo / use case (30-40 s)
6. Before / after or comparison (40-48 s)
7. User reaction / testimonial (48-55 s)
8. CTA (55-60 s)

### Section 7 — Text on screen

| Field | Type | Example |
| --- | --- | --- |
| `lines` | list[string] | ["MA5", "Mở hộp", "Trải nghiệm AR", "Giá ưu đãi"] |
| `keywords` | list[string] | ["AR", "kính thông minh", "made in VN"] |
| `price_offer` | string | "Pre-order giảm 15%" |

### Section 8 — Shoot notes

| Field | Type | Example |
| --- | --- | --- |
| `pitfalls` | list[string] | Avoid finger prints on lenses; keep background uncluttered |
| `must_see_details` | list[string] | Charging port, nose pad adjustment, LED indicator |
| `backup_shots` | list[string] | Static product hero shot, over-shoulder wear shot |
| `per_scene_time` | dict | {"hook": 3, "unboxing": 7, ...} |
| `priority` | string | Lens clarity and wearing comfort are top priority |

### Section 9 — Pre-shoot checklist

Boolean checklist. All must be true before shooting:

- [ ] Hook is clear and under 3 seconds.
- [ ] Problem / desire is stated in the first 5 seconds.
- [ ] Benefit is demonstrated, not just claimed.
- [ ] Demo shows the product in real use.
- [ ] CTA tells the viewer exactly what to do.
- [ ] Setting, angle, props, and timeline are locked.
- [ ] "Đọc kịch bản là triển khai" — any shooter can execute from this document.

## Example Filled Script — MA5 (UGC unboxing)

### 1. Overview

- **product:** MA5
- **goal:** Generate pre-order signups and social shares among Vietnamese tech
  creators.
- **audience:** Tech creators and early adopters, 18-35, active on TikTok and
  Facebook Reels.
- **platform:** TikTok + Facebook Reels.
- **duration:** 45 seconds.
- **format:** UGC-style unboxing + quick demo.

### 2. Shoot requirements

- **aspect_ratio:** 9:16
- **resolution_fps:** 1080p60
- **tone_mood:** Curious, energetic, premium-but-accessible.

### 3. Setting

- **location:** Clean white desk near a large window.
- **time:** Late morning, soft natural light.
- **lighting:** Key light 45° camera left, natural window fill on the right.
- **rationale:** Shows the product clearly without a sterile studio look.

### 4. Props & wardrobe

- **props:** MA5 unit, charging cable, branded cleaning cloth, phone showing
  companion app.
- **wardrobe:** Solid black T-shirt, no visible logos.

### 5. Timeline & shot list

| # | Duration | Purpose | Dialogue | Action | Angle | B-roll | On-screen text |
|---|----------|---------|----------|--------|-------|--------|----------------|
| 1 | 3s | Hook | "Chiếc kính AR này nhỏ gọn đến mức không tin nổi." | Hands place MA5 box on desk | Top-down | Box logo close-up | "MA5 — kính AR VN" |
| 2 | 5s | Unboxing | "Cùng mở hộp xem bên trong có gì." | Open box, reveal layers | Top-down | Slow peel of seal | "Unboxing" |
| 3 | 7s | Feature highlight | "Trọng lượng chỉ bằng một chiếc kính mát." | Pick up MA5, rotate | 45° side | Close-up of hinge | "Siêu nhẹ" |
| 4 | 10s | Demo | "Đeo vào, mở app, thế giới AR hiện ra ngay." | Put on MA5, interact with phone | POV + front | Screen reflection | "Trải nghiệm AR" |
| 5 | 8s | Before/after | "Trước đây phải cầm điện thoại, giờ chỉ cần nhìn." | Split screen: phone vs MA5 | Split | Hand holding phone | "Hands-free" |
| 6 | 7s | Reaction | "Thực sự mượt, không bị chóng mặt." | Smile, nod | Front cam | Close-up face | "User reaction" |
| 7 | 5s | CTA | "Link pre-order trong bio — giảm 15% tuần này." | Point to caption/link | Front cam | Product hero shot | "Pre-order -15%" |

### 6. Scenes

1. Hook — box reveal.
2. Product close-up — pick up and rotate MA5.
3. Feature highlight — weight and hinge.
4. Operation — wear + open app.
5. Demo — hands-free AR view.
6. Before/after — phone vs MA5.
7. User reaction — authentic smile and nod.
8. CTA — pre-order link.

### 7. Text on screen

- **lines:** ["MA5", "Kính AR Made in VN", "Siêu nhẹ", "Hands-free", "Pre-order -15%"]
- **keywords:** ["AR", "kính thông minh", "MA5", "Made in VN"]
- **price_offer:** "Pre-order giảm 15% trong tuần đầu."

### 8. Shoot notes

- **pitfalls:** Tránh dấu vân tay trên tròng kính; giữ nền gọn gàng.
- **must_see_details:** Cổng sạc, phần đệm mũi điều chỉnh, đèn LED báo trạng thái.
- **backup_shots:** Cảnh sản phẩm tĩnh hero; cảnh đeo qua vai.
- **per_scene_time:** hook 3s, unboxing 5s, feature 7s, demo 10s, comparison 8s,
  reaction 7s, CTA 5s.
- **priority:** Độ sáng và rõ nét của tròng kính là ưu tiên số 1.

### 9. Pre-shoot checklist

- [x] Hook under 3 seconds.
- [x] Problem stated in first 5 seconds.
- [x] Benefit demonstrated.
- [x] Demo shows real use.
- [x] CTA is specific.
- [x] Setting, angle, props, timeline locked.
- [x] Script is executable without further explanation.

## Hook Iteration Loop

1. Generate 3 hook variants that fit the product + format + platform.
2. If `mcp-trend-research` is configured, call `search_hooks` for the product
   category and the current month. If it is not ready, skip with a note — do
   not block the pipeline.
3. Score each hook against `mcp-ledger.get_hooks_leaderboard` historical
   performance. Prefer hooks whose style/keywords previously outperformed the
   median for this product category.
4. Pick the best-scoring hook. If two are tied, prefer the shorter one.
5. Record the runner-up hooks in the piece record so the manager can request a
   swap during revision.

## Manager Review Gate → Telegram

When a piece reaches `MANAGER_REVIEW_GATE`:

1. Build a concise summary:
   - Product name and SKU.
   - Recommended format(s).
   - Best hook.
   - 3-bullet script summary.
   - Full script link/dashboard reference (or inline text if no dashboard yet).
2. Call `messages_send` with target
   `telegram:<MANAGER_TELEGRAM_CHAT_ID>`.
3. Attach an inline keyboard with two callback payloads:
   - `marketing:approve:<piece_id>`
   - `marketing:revise:<piece_id>`
4. Wait for the callback. Do not auto-advance.
5. On approve: transition to `SHOOT_BRIEF`.
6. On revise: append manager feedback, return to `SCRIPT_DRAFT`, and regenerate
   the script.

The same flow applies to `FINAL_REVIEW_GATE` with callback payloads
`marketing:final_approve:<piece_id>` and `marketing:final_revise:<piece_id>`.

## MCP Tool Call Map

| State | MCP tool(s) / Hermes tool |
| --- | --- |
| PRODUCT_SELECT | `mcp-catalog.list_products`, `mcp-catalog.get_product_specs` |
| FORMAT_PICK | Agent LLM (uses format picker table) |
| SCRIPT_DRAFT | Agent LLM + `mcp-ledger.get_hooks_leaderboard` |
| HOOK_ITERATE | `mcp-trend-research.search_hooks` (Phase 5, gate if missing), `mcp-ledger.get_hooks_leaderboard` |
| MANAGER_REVIEW_GATE | `messages_send` to Telegram |
| SHOOT_BRIEF | Agent LLM (formats brief) |
| FOOTAGE_INGEST | `mcp-video-edit.cut_by_shotlist` |
| AUTO_EDIT | `mcp-video-edit.encode_916`, `mcp-video-edit.burn_captions`, `mcp-video-edit.add_music` |
| FINAL_REVIEW_GATE | `messages_send` to Telegram |
| PUBLISH | `mcp-social-youtube.upload`, `mcp-social-meta.post_reel`, `mcp-social-tiktok.post_video`, `mcp-ledger.record_post` |
| MONITOR | `mcp-social-*.get_stats` |
| REPLY/QUEUE | `mcp-social-*.list_comments`, `mcp-social-*.reply_comment`, `messages_send` |
| ANALYZE | `mcp-ledger.get_performance`, `mcp-ledger.query_what_worked` |
| LEDGER | `mcp-ledger.record_post` (update) |

## Human-Gate Rule

There are exactly 3 gates that require human input. The agent must pause at each
and never skip:

1. `MANAGER_REVIEW_GATE` — script/hook approval.
2. `FINAL_REVIEW_GATE` — final rendered video approval.
3. `SHOOT_BRIEF` execution — a human crew must shoot the footage; the agent only
   produces the brief.

At a gate, set the piece state to the gate state, send the notification, log
that the agent is waiting, and stop. Do not synthesize a human decision.

## One-Shot Recipes

### Start a new MA5 piece from the CLI

```bash
hermes marketing new MA5
```

This runs PRODUCT_SELECT → FORMAT_PICK → SCRIPT_DRAFT → HOOK_ITERATE and stops
at MANAGER_REVIEW_GATE. A Telegram message is sent to the manager.

### Approve a piece from the CLI (callback fallback)

```bash
hermes marketing approve <piece_id>
```

Transitions the piece from MANAGER_REVIEW_GATE to SHOOT_BRIEF.

### Publish an approved piece

```bash
hermes marketing publish <piece_id>
```

Runs PUBLISH state and records the post.

### Check pipeline status

```bash
hermes marketing status <piece_id>
```

Shows current state and full transition history.

## Verification Checklist

- [ ] `skills/marketing/marketing-pipeline/SKILL.md` exists and is valid markdown.
- [ ] File is ≥ 500 lines and covers all 14 states.
- [ ] Format picker table covers all 9 products.
- [ ] 9-section script template has all fields with types.
- [ ] One complete example script is provided for MA5.
- [ ] Hook iteration loop is documented (gen 3 → trend → score → pick).
- [ ] Manager review gate specifies Telegram target and approve/revise callbacks.
- [ ] MCP tool call map is complete per state.
- [ ] Human-gate rule names the 3 gates and states the agent must pause.
