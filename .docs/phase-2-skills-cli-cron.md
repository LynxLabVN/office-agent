# Phase 2 — Skills + CLI + cron (both domains)

**Goal:** Both SKILL.md playbooks written, CLI commands wired, cron jobs
registered, manager-review gate delivers to Telegram.
**Depends on:** Phase 1 + 1b (MCPs must exist for skills to call).
**Duration:** ~5 days.
**Parallelizable with:** Phase 3 (Phase 3 MCPs are OAuth-gated, can start
once app reviews complete — independent of skills).

Read [`00-overview.md`](./00-overview.md) first for workspace structure and
conventions.

---

## 2.1 `skills/marketing/marketing-pipeline/SKILL.md`

**File:** `agent-core/skills/marketing/marketing-pipeline/SKILL.md`

**Structure (write each section fully — this is the agent's playbook):**

1. **Trigger:** `/marketing-pipeline` command or auto-trigger on "new video
   for <product>".
2. **State machine:** document all 14 states (from PLAN.md pipeline) with
   entry/exit conditions and which MCP tool to call at each transition.
3. **Format picker:** a decision table mapping product category → recommended
   format(s) with reasoning:
   - audio product (mic, bao đàn, UHF) → demo + comparison + testimonial
   - visual product (MA5, A14, AD35, A8, GX200, P011) → UGC + unboxing + BTS
   - new launch → short-form + storytelling
4. **9-section script template:** full template with field names, types, and
   an example filled script for one product (use MA5).
5. **Hook iteration loop:** generate 3 hook variants → optionally call
   `mcp-trend-research.search_hooks` (Phase 5, gate if not ready) → score
   hooks against `mcp-ledger.get_hooks_leaderboard` historical performance →
   pick best.
6. **Manager review gate:** on reaching `MANAGER_REVIEW_GATE`, call
   `messages_send` to Telegram with script summary + approve/revise inline
   buttons. Wait for human response before transitioning.
7. **MCP tool call map:** for each state, which MCP tool(s) to call:
   - PRODUCT_SELECT → `mcp-catalog.list_products`
   - SCRIPT_DRAFT → agent LLM + `mcp-ledger.get_hooks_leaderboard`
   - FOOTAGE_INGEST → `mcp-video-edit.cut_by_shotlist`
   - AUTO_EDIT → `mcp-video-edit.encode_916` + `burn_captions` + `add_music`
   - PUBLISH → `mcp-social-youtube.upload` (Phase 3) + `mcp-ledger.record_post`
   - ANALYZE → `mcp-ledger.get_performance` + `query_what_worked`
8. **Human-gate rule:** explicitly state the 3 gates that require human input
   and that the agent must pause (not skip) at each.

**Verify:**
- File exists at the path, is valid markdown, ≥500 lines.
- All 14 states documented with entry/exit + tool calls.
- Format picker table covers all 9 products.
- 1 full example script for MA5.

## 2.2 `skills/hr/recruitment/SKILL.md`

**File:** `agent-core/skills/hr/recruitment/SKILL.md`

**Structure:**
1. **Trigger:** `/hr-recruit` or "post job for <role>".
2. **State machine:** 11 states (from PLAN.md HR pipeline) with entry/exit +
   MCP tool calls.
3. **JD template:** fields = title, must-have skills, nice-to-have skills,
   exp level, salary range, location, benefits, VN-specific (probation ≤60
   days, SI/HC/HI notes). Include 1 filled example.
4. **CV scoring rubric:** explicit weights — skills 40%, relevant exp 30%,
   portfolio quality 20%, education 10%. Document how `mcp-cv-screen` maps
   to each. Document human-override path.
5. **Interview flow:** screen → technical → culture fit → offer. For each:
   question bank generator prompt, duration, decision criteria.
6. **Question bank generator:** per role/skill, agent generates 10 questions
   per round, human reviews.
7. **Stage transition + auto-comms templates:** for each transition, which
   template to send + via which channel:
   - applied → acknowledge (Zalo OA or email)
   - screened → shortlist or reject (Zalo OA or email)
   - interview → invite with Cal.com link (Phase 3 `mcp-schedule`)
   - offer → offer letter (Zalo OA attachment or email)
   Each template written in Vietnamese with placeholders.
8. **VN labor law basics:** probation max 60 days (Labor Code Art 27),
   contract types (indefinite vs definite vs seasonal), SI/HC/HI employer
   obligations. For offer drafting only — not legal advice.
9. **MCP tool call map:** per state, which tools.
10. **Human-gate rule:** JD review + interview decision/offer = pause, not skip.

**Verify:**
- File exists, valid markdown, ≥400 lines.
- All 11 states documented.
- JD template + 1 example.
- CV rubric with explicit weights.
- All comms templates in Vietnamese.

## 2.3 CLI commands

**File:** `agent-core/hermes_cli/marketing.py` + `agent-core/hermes_cli/hr.py`
(register in the existing CLI dispatcher — do NOT modify `run_agent.py`).

**Marketing commands:**
| Command | Action | MCP tools called |
| ------- | ------ | ---------------- |
| `hermes marketing new <product_sku>` | Start pipeline at PRODUCT_SELECT | `mcp-catalog.get_product_specs` |
| `hermes marketing review-queue` | List pieces at MANAGER_REVIEW_GATE | (reads pipeline state from memory) |
| `hermes marketing publish <piece_id>` | Execute PUBLISH state | `mcp-social-*.upload` + `mcp-ledger.record_post` |
| `hermes marketing status <piece_id>` | Show current state + history | (memory) |

**HR commands:**
| Command | Action | MCP tools called |
| ------- | ------ | ---------------- |
| `hermes hr new-job` | Open JD draft editor (interactive) | (agent LLM) |
| `hermes hr post <job_id>` | Post to own careers page + generate board text | `mcp-hr-data` |
| `hermes hr screen <job_id>` | Run CV screening on all applied candidates | `mcp-cv-screen.score_cv_against_jd` |
| `hermes hr schedule <candidate_id>` | Book interview slot | `mcp-schedule.book_slot` (Phase 3) |
| `hermes hr pipeline <job_id>` | Show candidates kanban as text table | `mcp-hr-data.list_applications` |

**Verify:**
- `hermes marketing new MA5` → starts pipeline, shows product specs + format
  recommendation.
- `hermes hr new-job` → opens interactive JD draft.
- `hermes marketing --help` and `hermes hr --help` list all subcommands.

## 2.4 Cron jobs

**File:** `agent-core/cron/jobs.toml` (append to existing — do not overwrite).

**Marketing cron:**
```toml
[[jobs]]
name = "nightly-analytics-pull"
schedule = "0 2 * * *"          # 02:00 daily
command = "hermes marketing pull-analytics"
# calls mcp-social-*.get_stats for all posts in last 24h → mcp-ledger

[[jobs]]
name = "weekly-performance-report"
schedule = "0 9 * * 1"          # 09:00 Monday
command = "hermes marketing report"
# calls mcp-ledger.query_what_worked → generates summary → Telegram

[[jobs]]
name = "scheduled-publishing"
schedule = "*/15 * * * *"       # every 15 min, check queue
command = "hermes marketing publish-queue"
# posts any pieces with scheduled_at <= now
```

**HR cron:**
```toml
[[jobs]]
name = "interview-reminder-24h"
schedule = "0 8 * * *"          # 08:00 daily
command = "hermes hr remind-interviews"
# sends reminder to candidate + interviewer 24h before scheduled interview

[[jobs]]
name = "followup-nudge-48h"
schedule = "0 10 * * *"
command = "hermes hr nudge-no-reply"
# if candidate hasn't replied in 48h, send one follow-up (max 1 nudge)
```

**Verify:**
- `hermes cron list` shows all 5 jobs.
- Manually trigger: `hermes cron run nightly-analytics-pull` → executes
  without error (even if no posts yet).
- Check logs show the job ran and called the right MCP tools.

## 2.5 Manager-review gate → Telegram

**Do:**
1. In `marketing.py`, at `MANAGER_REVIEW_GATE` transition, call existing
   `messages_send` tool with `target="telegram:<manager_chat_id>"`.
2. Message format: product name, format, hook, script summary, link to full
   script in dashboard (Phase 4) or inline text.
3. Include approve/revise inline keyboard (Telegram callback buttons).
4. On callback → resume pipeline with approved or revised script.
5. Same pattern for HR `JD_REVIEW_GATE` and `DECISION_GATE`.

**Config:** `MANAGER_TELEGRAM_CHAT_ID` in Hermes env.

**Verify:**
- Run `hermes marketing new MA5` through to `MANAGER_REVIEW_GATE` →
  Telegram message arrives.
- Click approve → pipeline advances to `SHOOT_BRIEF`.
- Click revise → pipeline returns to `SCRIPT_DRAFT` with feedback.

---

## Phase 2 exit criteria

- [ ] Both SKILL.md files exist with full content
- [ ] `hermes marketing --help` + `hermes hr --help` list all subcommands
- [ ] `hermes marketing new MA5` runs through to manager gate
- [ ] `hermes cron list` shows 5 jobs, `cron run` executes without error
- [ ] Manager-review Telegram notification + callback works
