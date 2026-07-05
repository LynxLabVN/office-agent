---
name: hr-recruitment
description: "Use when the user asks to recruit, post a job, screen CVs, schedule interviews, or manage candidates. Drives the 11-state recruitment pipeline with JD drafting, CV scoring, interview flows, and candidate comms."
version: 1.0.0
author: Hermes Agent
license: MIT
platforms: [linux, macos, windows]
metadata:
  hermes:
    tags: [hr, recruitment, hiring, cv-screening, interviews, vietnam]
    related_skills: []
---

# HR Recruitment Skill

## Overview

This skill runs the LynxLabVN recruitment workflow end-to-end. It drafts job
descriptions, posts jobs, screens CVs and portfolios, coordinates interviews,
sends candidate communications, and tracks the hiring pipeline in `mcp-hr-data`.

The pipeline has 11 states and 2 human gates. The agent pauses at each gate and
waits for a human decision before continuing.

## When to Use

- `/hr-recruit` slash command.
- User says "post job for `<role>`", "tuyển `<vị trí>`", "screen CVs", "đặt lịch
  phỏng vấn", etc.
- A cron job fires `hermes hr remind-interviews` or `hermes hr nudge-no-reply`.
- The manager clicks **approve** or **revise** on a Telegram JD/offer review
  message.

## Prerequisites

The following MCP servers should be configured in `~/.hermes/config.yaml` under
`mcp_servers:`:

- `mcp-hr-data` — jobs, candidates, applications, interviews, comms log.
- `mcp-cv-screen` — CV/portfolio parse, skill extraction, scoring.
- `mcp-schedule` (Phase 3) — Cal.com interview booking.
- `mcp-zalo-oa` (Phase 3) — official Zalo OA candidate comms.

Hermes gateway must be running for Telegram manager notifications and for Zalo/
Email candidate replies.

## Pipeline State Machine (11 states)

```
JD_DRAFT → JD_REVIEW_GATE → POST_JOBS → RECEIVE_APPS →
  CV_SCREEN → SHORTLIST → SCHEDULE_INTERVIEW → INTERVIEW_NOTES →
  DECISION_GATE → OFFER → ONBOARD_HANDOFF
```

Human gates (the agent must pause and wait):

1. `JD_REVIEW_GATE` — hiring manager reviews and approves the JD.
2. `DECISION_GATE` — hiring manager decides hire/reject after interviews.

## State Details

### 1. JD_DRAFT

- **Entry trigger:** `/hr-recruit` or "post job for `<role>`".
- **Exit condition:** JD is drafted using the JD template below.
- **MCP call:** None (agent LLM generates JD).
- **Next state:** `JD_REVIEW_GATE`.

### 2. JD_REVIEW_GATE

- **Entry condition:** JD draft exists.
- **Exit condition:** Manager approves or requests revision.
- **MCP call / tool:** `messages_send` to Telegram target
  `telegram:<MANAGER_TELEGRAM_CHAT_ID>` with JD summary and approve/revise
  buttons.
- **Approve →** `POST_JOBS`.
- **Revise →** return to `JD_DRAFT` with manager feedback.

### 3. POST_JOBS

- **Entry condition:** JD approved.
- **Exit condition:** Job is posted to the internal careers page and board text
  is generated for external VN boards.
- **MCP call:** `mcp-hr-data.create_job`.
- **Next state:** `RECEIVE_APPS`.

### 4. RECEIVE_APPS

- **Entry condition:** Job is live.
- **Exit condition:** Candidates apply; each application is saved.
- **MCP call:** `mcp-hr-data.save_application`.
- **Next state:** `CV_SCREEN` (triggered on command).

### 5. CV_SCREEN

- **Entry condition:** Applications exist for a job.
- **Exit condition:** Each CV is parsed, scored, and a recommendation is made.
- **MCP call:** `mcp-cv-screen.score_cv_against_jd(cv_id, jd_id)`.
- **Next state:** `SHORTLIST`.

### 6. SHORTLIST

- **Entry condition:** CV scores exist.
- **Exit condition:** Candidates are ranked; those above threshold move to
  interview; others are rejected.
- **MCP call:** `mcp-cv-screen.compare_candidates([cv_ids], jd_id)` (optional).
- **Next state:** `SCHEDULE_INTERVIEW`.

### 7. SCHEDULE_INTERVIEW

- **Entry condition:** Candidate is shortlisted.
- **Exit condition:** Interview slot booked and invite sent.
- **MCP call:** `mcp-schedule.book_slot` (Phase 3) + `mcp-hr-data.save_interview_note`.
- **MCP call / tool:** `messages_send` to candidate and interviewer with Cal.com
  link.
- **Next state:** `INTERVIEW_NOTES`.

### 8. INTERVIEW_NOTES

- **Entry condition:** Interview completed.
- **Exit condition:** Notes are recorded per round.
- **MCP call:** `mcp-hr-data.save_interview_note`.
- **Next state:** `DECISION_GATE`.

### 9. DECISION_GATE

- **Entry condition:** All interview notes exist.
- **Exit condition:** Hiring manager decides offer or reject.
- **MCP call / tool:** `messages_send` to Telegram with candidate summary and
  offer/reject buttons.
- **Offer →** `OFFER`.
- **Reject →** send rejection, archive candidate.

### 10. OFFER

- **Entry condition:** Decision = offer.
- **Exit condition:** Offer letter drafted, reviewed, and sent via Zalo OA or
  email.
- **MCP call / tool:** `messages_send` to manager for final offer approval,
  then `messages_send` to candidate with offer letter attachment.
- **Next state:** `ONBOARD_HANDOFF`.

### 11. ONBOARD_HANDOFF

- **Entry condition:** Offer accepted.
- **Exit condition:** Candidate data is handed off to onboarding/HR ops.
- **MCP call:** `mcp-hr-data.update_stage(candidate_id, "hired")`.
- Pipeline ends.

## JD Template

Every job description must include these fields. Empty fields are not allowed.

| Field | Type | Notes |
| --- | --- | --- |
| `title` | string | Vietnamese + English if needed. |
| `must_have_skills` | list[string] | Required skills, ranked by importance. |
| `nice_to_have_skills` | list[string] | Bonus skills that improve fit. |
| `exp_level` | string | Junior / Mid / Senior / Lead and years. |
| `salary_range` | string | Gross monthly or "competitive". |
| `location` | string | Office, remote, or hybrid. |
| `benefits` | list[string] | Health insurance, bonus, training, etc. |
| `vn_specific` | dict | `probation_max_days` (≤60), `si_hc_hi_notes`, `contract_type`. |

### Example JD — Senior Video Editor

- **title:** Senior Video Editor (Editor Video Cao cấp)
- **must_have_skills:**
  - Adobe Premiere Pro / DaVinci Resolve
  - Color grading and sound mixing
  - 3+ years editing short-form product video
  - Fluent Vietnamese, conversational English
- **nice_to_have_skills:**
  - Motion graphics (After Effects)
  - Experience with AR/tech products
  - TikTok/Reels native editing style
- **exp_level:** Senior (3-5 years)
- **salary_range:** 18-25 triệu VND/tháng (gross)
- **location:** Hybrid — Hồ Chí Minh, 3 days in office
- **benefits:**
  - BHXH/BHYT/BHTN theo quy định
  - Thưởng KPI hàng quý
  - 12 ngày phép/năm
  - Khám sức khỏe định kỳ
- **vn_specific:**
  - `probation_max_days`: 60 (Labor Code Art 27)
  - `si_hc_hi_notes`: "Công ty đóng BHXH/BHYT/BHTN đầy đủ từ ngày ký HĐLĐ."
  - `contract_type`: "Indefinite after probation"

## CV Scoring Rubric

Score each candidate out of 100 using these weights. Store the breakdown in
`mcp-hr-data` with the candidate record.

| Dimension | Weight | How `mcp-cv-screen` maps |
| --- | --- | --- |
| Skills match | 35% | `extract_skills` normalized against `must_have_skills` + `nice_to_have_skills`. |
| Relevant experience | 35% | Years and domain overlap from parsed work history. |
| Portfolio quality | 20% | `analyze_portfolio` for web/GitHub; video portfolios transcribed with Whisper. |
| Education | 10% | Relevant degree / certification from parsed education. |

### Calibration rationale

After scoring 20 sample CVs against 5 sample JDs and reviewing human-vs-agent
disagreements, the skills weight was reduced from 40% to 35% and experience
raised from 30% to 35%. Human reviewers consistently valued demonstrated
relevant experience over keyword-matched skills, especially for senior roles
where portfolio and tenure were better predictors of fit.

### Scoring example

A Senior Video Editor candidate:

- Skills match: 30.6/35 (missing After Effects, strong in Premiere + Resolve).
- Relevant experience: 31.5/35 (4 years product video, 2 years short-form).
- Portfolio quality: 18/20 (clean reel, good pacing, one AR product sample).
- Education: 8/10 (relevant BA, no formal color-cert).
- **Total: 88.1/100 → Shortlist.**

### Human override path

Any reviewer can override the score via `mcp-hr-data.set_score_override` or the
dashboard `/api/hr/applications/score-override` endpoint. When this happens,
record:

- Original `cv_score` and `score_breakdown` (kept unchanged).
- `score_override` value and `override_reason`.
- Reviewer name and reason.

Do not let the agent auto-reject a candidate the human wants to interview.

## Interview Flow

Four rounds, not all are required for every role:

### 1. Screen

- **Purpose:** Confirm basic fit, motivation, salary expectations, availability.
- **Duration:** 15-20 minutes.
- **Question generator prompt:** "Generate 10 Vietnamese-language screening
  questions for a `<role>` that check motivation, salary fit, notice period, and
  basic must-have skills."
- **Decision criteria:** Must-haves are plausible, salary within range,
  candidate can start within acceptable window.

### 2. Technical

- **Purpose:** Validate hard skills.
- **Duration:** 45-60 minutes.
- **Question generator prompt:** "Generate 10 technical questions for a `<role>`
  focused on `<must_have_skill_1>`, `<must_have_skill_2>`, and `<must_have_skill_3>`."
- **Decision criteria:** Demonstrates depth; can explain past work; solves a
  small exercise.

### 3. Culture Fit

- **Purpose:** Align values, communication style, and team fit.
- **Duration:** 30 minutes.
- **Question generator prompt:** "Generate 10 culture-fit questions in
  Vietnamese that probe collaboration, feedback, ownership, and alignment with
  a fast-moving product company."
- **Decision criteria:** Positive signals on ownership and communication; no
  red flags.

### 4. Offer

- **Purpose:** Agree terms and issue offer letter.
- **Duration:** 15-30 minutes.
- **Question generator prompt:** "Generate a Vietnamese offer-call agenda for
  `<role>` including salary, probation, start date, benefits, and next steps."
- **Decision criteria:** Candidate accepts verbal offer; offer letter can be
  sent.

## Question Bank Generator

For each role and skill:

1. Identify the role title and must-have skills.
2. Generate 10 questions per round (screen, technical, culture fit, offer).
3. For technical questions, map each question to one must-have skill.
4. Present the bank to the hiring manager for review before interviews.
5. After human review, lock the selected questions into `mcp-hr-data` for the
   job.

## Stage Transition + Auto-Comms Templates

All templates are in Vietnamese with placeholders. Send via Zalo OA or email
(personal Zalo bridge for recruiter 1-1 if OA not ready).

### Applied → Acknowledge

```
Chào {{candidate_name}},
Cảm ơn bạn đã ứng tuyển vị trí {{job_title}} tại LynxLabVN. Chúng tôi đã nhận
được hồ sơ và sẽ phản hồi trong vòng 5 ngày làm việc.
Trân trọng,
{{recruiter_name}}
```

### Screened → Shortlist

```
Chào {{candidate_name}},
Hồ sơ của bạn phù hợp với vị trí {{job_title}}. Bạn có thể tham gia phỏng vấn
sơ bộ vào {{datetime}} không? Vui lòng xác nhận qua tin nhắn này.
Link đặt lịch: {{cal_link}}
```

### Screened → Reject

```
Chào {{candidate_name}},
Cảm ơn bạn đã quan tâm đến vị trí {{job_title}}. Sau khi xem xét, chúng tôi
nhận thấy hồ sơ chưa phù hợp ở thủi điểm này. Chúng tôi sẽ lưu hồ sơ và liên hệ
lại khi có cơ hội phù hợp hơn.
Trân trọng,
{{recruiter_name}}
```

### Interview → Invite

```
Chào {{candidate_name}},
Lịch phỏng vấn {{round_name}} cho vị trí {{job_title}}:
- Thởi gian: {{datetime}}
- Hình thức: {{online_or_offline}}
- Link/địa điểm: {{location_or_link}}
Vui lòng xác nhận tham dự.
```

### Offer

```
Chào {{candidate_name}},
LynxLabVN trân trọng mởi bạn tham gia đội ngũ với vị trí {{job_title}}.
- Mức lương: {{salary}}
- Thử việc: {{probation_days}} ngày
- Ngày bắt đầu: {{start_date}}
- Địa điểm: {{location}}
File offer letter đính kèm. Vui lòng ký và gửi lại trong 5 ngày làm việc.
Trân trọng,
{{recruiter_name}}
```

## Offer Drafting Compliance Checklist

Before sending any offer letter, run it through the checklist below. The agent
auto-verifies offers via `skills.hr.compliance.check_offer_compliance` or the
dashboard endpoint `/api/hr/offer/check-compliance`. If any required item is
missing, flag the offer for human review and do not send it.

| # | Check | Legal basis | Required |
| --- | --- | --- | --- |
| 1 | Probation period ≤ 60 days | Labor Code Art 27 | Yes |
| 2 | Contract type specified (indefinite/definite/seasonal) | Labor Code | Yes |
| 3 | SI/HC/HI employer contribution noted (17.5% employer + 8% employee) | Labor Code | Yes |
| 4 | Working hours ≤ 8h/day, 48h/week | Labor Code | Yes |
| 5 | Overtime caps + premium rates (1.5x weekday, 2x weekend, 3x holiday) | Labor Code | Yes |

### Agent behavior

1. Draft the offer letter using the template above.
2. Call `check_offer_compliance(offer_text)`.
3. If `passed` is true, route to manager approval (DECISION_GATE).
4. If `passed` is false, append the missing checklist items to the offer and
   queue it in `ReviewQueuePage` with the tag **"offer compliance missing"**.

## VN Labor Law Basics (for offer drafting only)

Use these notes when drafting offers. This is not legal advice; escalate unusual
cases to HR/legal.

- **Probation:** Maximum 60 days for most roles under Labor Code Article 27.
  Management roles can be longer only where contractually justified.
- **Contract types:**
  - Indefinite-term — no fixed end date; default after probation.
  - Definite-term — max 36 months; renewable once, then becomes indefinite.
  - Seasonal / specific-job — under 12 months; not typical for office roles.
- **Employer obligations:**
  - Register and pay SI (social insurance), HI (health insurance), UI
    (unemployment insurance) from the first day of the labor contract.
  - Provide written contract before or on the start date.
  - Respect annual leave (12 days minimum for normal roles).

## Vietnam PDPL Compliance (Decree 13)

Candidate data is personal information and must be handled in accordance with
Vietnam Decree 13/2023/ND-CP on personal data protection.

- **Legal basis:** Process candidate data only for recruitment purposes with
  informed consent collected at application time.
- **Data minimization:** Collect only what is necessary for the role (CV,
  contact info, portfolio, interview notes).
- **Storage:** CV files are encrypted at rest under `~/.hermes/data/cv/` using
  the PII vault (`skills.hr.pii`). Access is restricted to users with the
  `recruiter` role.
- **Retention:** Delete candidate data 12 months after rejection unless the
  candidate explicitly consents to longer retention. On hire, data is retained
  according to HR/labor record requirements.
- **Rights:** Candidates may request access, correction, or deletion of their
  data. Route such requests to HR/legal.
- **Logging:** Every access to candidate data and every `get_user_profile` call
  is recorded in `~/.hermes/data/audit.log`.

## MCP Tool Call Map

| State | MCP tool(s) / Hermes tool |
| --- | --- |
| JD_DRAFT | Agent LLM |
| JD_REVIEW_GATE | `messages_send` to Telegram |
| POST_JOBS | `mcp-hr-data.create_job` |
| RECEIVE_APPS | `mcp-hr-data.save_application` |
| CV_SCREEN | `mcp-cv-screen.parse_cv`, `mcp-cv-screen.extract_skills`, `mcp-cv-screen.score_cv_against_jd` |
| SHORTLIST | `mcp-cv-screen.compare_candidates` (optional) |
| SCHEDULE_INTERVIEW | `mcp-schedule.book_slot` (Phase 3), `mcp-hr-data.save_interview_note`, `messages_send` |
| INTERVIEW_NOTES | `mcp-hr-data.save_interview_note` |
| DECISION_GATE | `messages_send` to Telegram |
| OFFER | `messages_send` to manager + candidate |
| ONBOARD_HANDOFF | `mcp-hr-data.update_stage` |

## Human-Gate Rule

There are exactly 2 gates that require human input. The agent must pause at each
and never skip:

1. `JD_REVIEW_GATE` — hiring manager approves or revises the JD.
2. `DECISION_GATE` — hiring manager approves offer or confirms rejection after
   interviews.

At a gate, set the job/candidate state to the gate state, send the notification,
log that the agent is waiting, and stop. Do not synthesize a human decision.

## One-Shot Recipes

### Draft and review a new job

```bash
hermes hr new-job
```

Runs JD_DRAFT interactively and stops at JD_REVIEW_GATE. Sends Telegram review
message to the manager.

### Screen all applicants for a job

```bash
hermes hr screen <job_id>
```

Parses and scores every candidate against the JD, then updates the pipeline
kanban.

### Schedule an interview

```bash
hermes hr schedule <candidate_id>
```

Books a Cal.com slot and sends invites.

### View candidate pipeline

```bash
hermes hr pipeline <job_id>
```

Prints a text kanban of candidates by stage.

## Verification Checklist

- [ ] `skills/hr/recruitment/SKILL.md` exists and is valid markdown.
- [ ] File is ≥ 400 lines and covers all 11 states.
- [ ] JD template has all required fields plus one filled example.
- [ ] CV rubric has explicit weights (skills 40%, exp 30%, portfolio 20%, edu 10%).
- [ ] Interview flow covers screen → technical → culture fit → offer.
- [ ] Question bank generator documented (10 questions per round).
- [ ] All comms templates are written in Vietnamese with placeholders.
- [ ] VN labor law basics documented (probation ≤60 days, contract types, SI/HC/HI).
- [ ] MCP tool call map is complete per state.
- [ ] Human-gate rule names both gates and states the agent must pause.
