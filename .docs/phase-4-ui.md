# Phase 4 — UI redesign (additive)

**Goal:** 8 marketing + 6 HR dashboard pages, domain switcher, all API
endpoints, build passes.
**Depends on:** Phase 1 + 1b + 2 (API endpoints wrap MCP tools that must
exist) + Phase 3 for publish/analytics pages (can stub if not ready).
**Duration:** ~1.5 weeks.
**Parallelizable with:** Phase 5.

Read [`00-overview.md`](./00-overview.md) first for workspace structure and
conventions.

---

## 4.0 Open Design (optional accelerator)

Open Design can generate the dashboard pages in this phase as runnable HTML
artifacts. The detailed install commands, ready-to-paste prompts for all 14
pages, and handoff checklist live in a separate reference file:

**[`phase-4-opendesign-prompt.md`](./phase-4-opendesign-prompt.md)**

Quick start:

```bash
# Install Open Design into OpenCode
curl -fsSL https://open-design.ai/install.sh | sh -s opencode
od mcp install opencode

# Generate one page
od plugin apply example-dashboard \
  --input brief="Marketing analytics dashboard with views/likes chart and hooks leaderboard" \
  --output ./.design-output/marketing-analytics.html
```

---

## 4.1 Backend API endpoints

**File:** `agent-core/hermes_cli/web_server.py` (extend — add routers, do not
modify existing routes).

**Pattern:** each endpoint is a thin wrapper that calls the corresponding MCP
tool via the Hermes internal MCP client and returns JSON. No business logic
in the web server.

**Marketing endpoints (add under `/api/marketing/*` — note: prefixed to avoid
collision with existing `/api/*`):**

| Endpoint | Method | Body/Query | Calls MCP tool |
| -------- | ------ | ---------- | -------------- |
| `/api/marketing/pipeline/items` | GET | `?state?` | reads from memory/ledger |
| `/api/marketing/pipeline/move` | POST | `{piece_id, new_state}` | updates pipeline state |
| `/api/marketing/script/save` | POST | `{piece_id, script_json}` | saves to memory |
| `/api/marketing/script/generate` | POST | `{product_sku, format}` | agent LLM + `mcp-catalog` |
| `/api/marketing/hooks/suggest` | POST | `{product_sku, format}` | `mcp-ledger.get_hooks_leaderboard` + LLM |
| `/api/marketing/review/pending` | GET | — | reads pipeline for MANAGER_REVIEW_GATE |
| `/api/marketing/review/decide` | POST | `{piece_id, decision, feedback?}` | advances pipeline |
| `/api/marketing/edit/ingest` | POST | `{piece_id, footage_path}` | `mcp-video-edit.cut_by_shotlist` |
| `/api/marketing/edit/render` | POST | `{piece_id, shotlist, captions, music?}` | `mcp-video-edit.*` chain |
| `/api/marketing/edit/preview` | GET | `?piece_id` | serves preview video |
| `/api/marketing/publish/schedule` | POST | `{piece_id, platforms[], scheduled_at}` | queues |
| `/api/marketing/publish/now` | POST | `{piece_id, platforms[]}` | `mcp-social-*.upload` + `mcp-ledger.record_post` |
| `/api/marketing/comments/list` | GET | `?platform?` | `mcp-social-*.list_comments` |
| `/api/marketing/comments/reply` | POST | `{comment_id, platform, text}` | `mcp-social-*.reply` (gated by reply policy) |
| `/api/marketing/analytics/overview` | GET | `?date_from?&date_to?` | `mcp-ledger.query_what_worked` |
| `/api/marketing/analytics/drilldown` | GET | `?group_by&product?&format?` | `mcp-ledger.query_what_worked` |
| `/api/marketing/catalog` | GET/POST/PUT/DELETE | product CRUD | `mcp-catalog.*` |

**HR endpoints (under `/api/hr/*`):**

| Endpoint | Method | Body/Query | Calls MCP tool |
| -------- | ------ | ---------- | -------------- |
| `/api/hr/jobs` | GET, POST | — / job JSON | `mcp-hr-data.list_jobs` / `create_job` |
| `/api/hr/jobs/{id}` | GET, PUT, DELETE | — / job JSON | `mcp-hr-data.get_job` |
| `/api/hr/jobs/{id}/post` | POST | `{boards[]}` | generates board text + posts to own careers page |
| `/api/hr/pipeline` | GET | `?job_id` | `mcp-hr-data.list_applications` |
| `/api/hr/pipeline/move` | POST | `{application_id, new_stage}` | `mcp-hr-data.update_stage` |
| `/api/hr/candidates` | GET | — | `mcp-hr-data` candidate list |
| `/api/hr/candidates/{id}` | GET | — | `mcp-hr-data.get_candidate` |
| `/api/hr/candidates/{id}/cv` | GET | — | serves CV file |
| `/api/hr/compare` | POST | `{candidate_ids[], job_id}` | `mcp-cv-screen.compare_candidates` |
| `/api/hr/screen` | POST | `{job_id}` | `mcp-cv-screen.score_cv_against_jd` for all applied |
| `/api/hr/schedule/slots` | GET | `?event_type_id&date` | `mcp-schedule.list_slots` |
| `/api/hr/schedule/book` | POST | `{event_type_id, start, candidate_id}` | `mcp-schedule.book_slot` |
| `/api/hr/comms/log` | GET | `?candidate_id?&channel?` | reads `comms_log` via `mcp-hr-data` |
| `/api/hr/comms/send` | POST | `{candidate_id, channel, text, template?}` | `messages_send` + logs to `comms_log` |
| `/api/hr/analytics/overview` | GET | `?date_from?&date_to?` | aggregates from `mcp-hr-data` |

**Verify:**
- `cd agent-core && python -m hermes_cli.web_server` starts.
- `curl localhost:8000/api/marketing/catalog` → returns product list.
- `curl localhost:8000/api/hr/jobs` → returns job list.
- Each endpoint has a unit test in `web_server_test.py` (mock MCP client).

## 4.2 Frontend pages — Marketing (8 pages)

**File:** `agent-core/web/src/pages/marketing/`

**Stack:** React 19 + Vite + Tailwind v4 + shadcn-style (match existing pages).
**API client:** extend `agent-core/web/src/lib/api.ts` with `marketing.*` and
`hr.*` namespaces.

| Page file | Route | Components |
| --------- | ----- | ---------- |
| `PipelinePage.tsx` | `/marketing/pipeline` | Kanban board, drag-drop columns = pipeline states, cards = pieces |
| `ScriptWorkspacePage.tsx` | `/marketing/script/:pieceId` | 9-section editor (left), hook generator side-panel (right) with trend refs |
| `ReviewQueuePage.tsx` | `/marketing/review` | List of pending reviews, inline approve/revise, Telegram route button |
| `EditPreviewPage.tsx` | `/marketing/edit/:pieceId` | Upload zone for raw footage, shotlist editor, render button, video preview |
| `PublishConsolePage.tsx` | `/marketing/publish/:pieceId` | Platform toggles (YT/IG/FB/TikTok), per-platform caption, schedule datetime |
| `CommentInboxPage.tsx` | `/marketing/comments` | Platform filter tabs, comment cards, AI-suggested reply, approve/send |
| `AnalyticsPage.tsx` | `/marketing/analytics` | Charts (recharts or visx): views/likes over time, hooks leaderboard table |
| `CatalogPage.tsx` | `/marketing/catalog` | Product table, add/edit modal, image upload |

**Verify:**
- `cd agent-core/web && npm run build` succeeds.
- Each page renders without console errors (manual check in browser).
- `PipelinePage` shows pieces (seeded from API).
- `CatalogPage` CRUD works (add a product → appears in list).

## 4.3 Frontend pages — HR (6 pages)

**File:** `agent-core/web/src/pages/hr/`

| Page file | Route | Components |
| --------- | ----- | ---------- |
| `HRJobsPage.tsx` | `/hr/jobs` | Job table, JD editor (markdown), post-to-boards console |
| `HRPipelinePage.tsx` | `/hr/pipeline/:jobId` | Kanban: Applied → Screened → Interview → Offer → Hired/Rejected |
| `HRCandidatesPage.tsx` | `/hr/candidates` | Candidate list, CV viewer (PDF embed), score breakdown bar, compare view |
| `HRSchedulePage.tsx` | `/hr/schedule` | Cal.com embed + slot picker + send invite button |
| `HRCommsPage.tsx` | `/hr/comms` | Thread view per candidate, channel filter (Zalo/Telegram/Email), template picker |
| `HRAnalyticsPage.tsx` | `/hr/analytics` | Time-to-hire, funnel conversion, source effectiveness charts |

## 4.4 Domain switcher + nav

**File:** `agent-core/web/src/App.tsx` (extend).

**Do:**
- Add top-level domain switcher: `Marketing | HR` (tab or route prefix).
- Route structure: `/marketing/*` and `/hr/*` + existing routes stay at root.
- Nav sidebar shows domain-specific links when that domain is active.
- Reuse existing shell, layout, tokens, components.

**Verify:**
- Switching domains updates nav + routes.
- Existing pages (Status, Config, Env, Chat) still accessible at their routes.

## 4.5 Build + deploy

**Do:**
```bash
cd agent-core/web
npm run build           # outputs to dist/
cp -r dist/* ../hermes_cli/web_dist/
```

**Verify:**
- `hermes web` serves the dashboard at `localhost:8000`.
- All 14 new pages reachable via nav.
- `curl localhost:8000/` returns the SPA.
- API endpoints from 4.1 respond.

---

## Phase 4 exit criteria

- [ ] `npm run build` succeeds, no TypeScript errors
- [ ] 14 new pages render in browser without console errors
- [ ] Domain switcher works, existing pages intact
- [ ] All API endpoints respond (test with curl)
- [ ] `hermes web` serves the full dashboard
