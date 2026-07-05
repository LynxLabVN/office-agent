# Phase 4 — Open Design prompt & command reference

Use this document as a standalone guide for generating the Phase 4 marketing
and HR dashboard UI with Open Design, independent of the main phase plan.

---

## What this is for

Phase 4 needs **8 marketing pages** and **6 HR pages** in a React + Vite +
Tailwind v4 app. Open Design can accelerate the work by producing runnable
HTML artifacts (dashboards, Kanbans, tables, forms) that engineering then ports
into the project.

This file contains:
1. Install / setup commands
2. MCP wiring for OpenCode
3. Skill/plugin discovery commands
4. One detailed master prompt describing the whole app and every page
5. Artifact handoff checklist
6. Verification commands

---

## Install Open Design

Pick one path.

### Path A — Desktop app (fastest, zero config)

Download the latest release for your platform:

- macOS Apple Silicon:
  https://github.com/nexu-io/open-design/releases/download/open-design-v0.13.0/open-design-0.13.0-mac-arm64.dmg
- macOS Intel:
  https://github.com/nexu-io/open-design/releases/download/open-design-v0.13.0/open-design-0.13.0-mac-x64.dmg
- Windows x64:
  https://github.com/nexu-io/open-design/releases/download/open-design-v0.13.0/open-design-0.13.0-win-x64-setup.exe
- Linux: AppImage or Docker Compose on the release page.

### Path B — CLI install script

```bash
curl -fsSL https://open-design.ai/install.sh | sh -s opencode
```

### Path C — Run from source

```bash
# Requirements: Node.js 24, pnpm 10.33.x, git
git clone https://github.com/nexu-io/open-design.git
cd open-design
corepack enable && pnpm install
pnpm tools-dev
# Daemon: http://127.0.0.1:17456
# Web UI:  http://127.0.0.1:17573
```

---

## Wire Open Design into OpenCode

```bash
od mcp install opencode
od mcp install opencode --print   # preview config without writing
od mcp uninstall opencode          # remove later if needed
```

After install, OpenCode can read Open Design skills, design systems, and
projects as MCP tools.

---

## Discover useful skills and plugins

```bash
# List skills by scenario
od skill list --scenario marketing
od skill list --scenario hr
od skill list --scenario operation

# Search the plugin catalog
od plugin search dashboard
od plugin search "landing page"
od plugin search kanban
od plugin search pipeline
od plugin search table

# Inspect before applying
od plugin info example-dashboard
od plugin info example-github-dashboard
od plugin info example-data-report
od plugin info example-flowai-live-dashboard-template
```

---

## Master prompt (copy-paste into Open Design)

Copy the single prompt below into the Open Design web UI chat, or pass it via
`od plugin apply`. It describes the whole app — what it is, what it contains,
and how every page should behave — so the agent can render any page in a
consistent system.

### The prompt

```text
Build the UI for "Office Agent" — an internal operations platform that runs
two teams out of one dashboard: a Marketing team that produces and publishes
social content, and an HR team that hires and communicates with candidates.
The app is a single-page web app (React 19 + Vite + Tailwind v4, shadcn-style
components) served by a Python backend at localhost:8000. Generate each screen
as a self-contained, responsive HTML artifact using Tailwind CSS, with a
persistent top bar and left sidebar. Light/dark mode toggle in the top bar.
Target desktop first (1280px–1440px), collapsing gracefully to tablet.

APP SHELL
- Top bar: app name "Office Agent", a two-tab domain switcher
  (Marketing | HR), a global search input, a notifications bell, and a
  light/dark toggle.
- Left sidebar: navigation links. The links change depending on the active
  domain (see below). Each link has an icon and a label. The active link is
  highlighted. The sidebar also shows the signed-in user's avatar at the
  bottom.
- Main content area: renders the page for the selected route.
- A "Settings"/gear group at the bottom of the sidebar (below the domain nav)
  opens the Agent Functions — the existing Hermes dashboard surface that runs
  the agent itself. These pages stay at the root level (not under /marketing
  or /hr) and must remain accessible regardless of which domain is active.

AGENT FUNCTIONS (existing Hermes dashboard — keep at root)
The app is not only Marketing + HR; it also runs and configures the agent.
Keep the full set of existing agent-management pages at root routes, grouped
under a "Settings" section in the sidebar. Each page below must exist.

UI/UX CUSTOMIZATION
- Theme switcher (in the top bar, next to the dark toggle): a dropdown listing
  the 8 built-in dashboard themes — Hermes Teal (default), Hermes Teal Large,
  Nous Blue (light), Midnight, Ember, Mono, Cyberpunk, Rosé. Switching a theme
  changes the whole app's palette, fonts, corner radius, and density live,
  without a reload. Include a small swatch preview next to each theme name.
- Font + density controls (inside the System or Config page): let the user
  pick the sans/mono font family, base font size, line height, letter
  spacing, corner radius, and density (comfortable / spacious). These map to
  the theme's typography + layout tokens.
- Profile switcher (in the sidebar footer, above the user avatar): a dropdown
  to switch between isolated profiles (each profile = its own config, keys,
  sessions, skills, memory). Shows the active profile name + a "New profile"
  button that opens the Profile Builder.

AGENT PAGES (root routes, each with sidebar link + icon)
- Chat  (/chat) — an embedded terminal chat with the agent. Render it as a
  dark terminal pane (monospace) with a composer at the bottom, a session
  list on the left, and slash-command autocomplete. This is the primary
  conversation surface; do not replace it with a generic chat widget.
- Sessions (/sessions) — searchable list of past sessions with title,
  preview, timestamp, and delete; click to resume.
- Files (/files) — a file browser over the agent's working directory: tree
  on the left, file contents on the right, with read/edit.
- Analytics (/analytics) — token-usage dashboard: input vs output tokens
  per session/model, cost estimates, charts over time. (Distinct from the
  Marketing analytics page — this is agent telemetry.)
- Models (/models) — model + provider configuration: pick the active
  provider/model, view context length, set auxiliary models per task
  (title, vision, embedding, curator), test a model.
- Logs (/logs) — a log viewer for agent.log / errors.log / gateway.log with
  level filter, follow mode, and per-session filter.
- Cron (/cron) — scheduled jobs: list, add, edit, pause, resume, run,
  remove. Show schedule, next fire, last run status.
- Skills (/skills) — skill management: list installed skills, view
  SKILL.md, enable/disable, install from the hub, run/salvage.
- Plugins (/plugins) — plugin management: list installed plugins, enable/
  disable, configure, install/uninstall.
- MCP (/mcp) — MCP server catalog: list connected MCP servers, their tools,
  status; add/remove servers.
- Channels (/channels) — messaging channels: status of each platform
  adapter (Telegram, Discord, Slack, WhatsApp, …), connect/disconnect,
  per-channel config.
- Webhooks (/webhooks) — webhook endpoints: list, add, remove, view recent
  deliveries.
- Pairing (/pairing) — device/session pairing: list paired users across
  platforms, pair a new device, revoke.
- Profiles (/profiles) — profile management: list, switch, clone, delete.
  /profiles/new opens the Profile Builder (name, clone-from, model, tools).
- Config (/config) — a structured editor for config.yaml: sections for
  model, agent, terminal, memory, security, delegation, gateway, logging,
  cron, skills, plugins. Form fields, not raw YAML.
- Keys (/env) — a secrets manager for .env: list API keys/tokens, add/edit/
  remove, mask values, show only on edit. Separate OAuth provider logins
  section with "Copy command" to set up via CLI.
- System (/system) — system status + health: gateway status, active
  sessions, version, update check, restart gateway, update Hermes.
- Docs (/docs) — an embedded docs viewer (iframe to the docs site).

These agent pages use the same shell, the same theme, and the same component
primitives as the Marketing and HR pages — one app, three nav groups
(Marketing | HR | Settings/Agent).

DOMAIN: MARKETING
The Marketing domain manages the full lifecycle of a content "piece" — from
idea, to script, to footage, to edit, to manager review, to publish, to
comments, to analytics — plus a product catalog that feeds script generation.
Sidebar links (in order): Pipeline, Review Queue, Comments, Analytics, Catalog.
The Script, Edit, and Publish pages are reached by opening a piece from the
Pipeline and are scoped to that piece (route includes :pieceId).

Marketing page 1 — Pipeline (Kanban)
Route: /marketing/pipeline
A drag-and-drop Kanban board. Columns are the pipeline states, left to right:
Idea, Scripting, Recording, Editing, Manager Review, Scheduled, Published.
Each card represents one content piece and shows: thumbnail image, title,
format badge (one of YT / IG / FB / TikTok), owner avatar, due date, and a
small state indicator. Dragging a card between columns updates its state.
Clicking a card opens the piece (routes to Script, Edit, or Publish depending
on current state). A top filter bar lets you filter by state, format, and
owner. A "New piece" button creates a card in the Idea column.

Marketing page 2 — Script Workspace (per piece)
Route: /marketing/script/:pieceId
Two-column layout. Left column (60%): a 9-section script editor, each section
a labeled textarea — Hook, Problem, Solution, Product Demo, Proof/Testimonial,
Objection Handling, CTA, Offer Details, Outro. A "Generate" button at the top
of the column asks the agent to draft the 9 sections from the product SKU and
format. A "Save" button persists. Right column (40%): a Hook Generator
side-panel. It shows a list of trending hooks for the chosen format, each with
a short reference link and a "Use" button that inserts the hook into section 1.
Bottom action bar: Save, Generate, Preview (opens Edit page).

Marketing page 3 — Review Queue
Route: /marketing/review
A list of pieces currently in the "Manager Review" state. Each row shows:
thumbnail, title, submitter name + avatar, submitted date, and two buttons —
Approve and Revise. Clicking Revise expands an inline feedback textarea and
a "Send back" button. A "Route to Telegram" button on each row forwards the
review to a manager via Telegram. Filter by submitter and date range at top.

Marketing page 4 — Edit & Preview (per piece)
Route: /marketing/edit/:pieceId
Top: a dropzone to upload raw footage (drag-drop or click). Below: a shotlist
editor — a table with columns Shot #, In-point, Out-point, Description,
Duration. Add/remove rows. A captions editor (textarea per shot). An optional
music track selector (dropdown of licensed tracks). A "Render" button that
runs the render chain and shows progress. Bottom: a video preview player
(<video>) showing the rendered preview with play/pause/scrub and a
"Download MP4" link.

Marketing page 5 — Publish Console (per piece)
Route: /marketing/publish/:pieceId
Top: four platform toggle cards — YouTube, Instagram, Facebook, TikTok — each
toggleable on/off with the platform icon. For each enabled platform show a
caption textarea and a tags input. A scheduled-at datetime picker
(date + time). Two action buttons: "Schedule" (queues for later) and
"Publish now" (uploads immediately). A small status line shows the last
publish result per platform.

Marketing page 6 — Comment Inbox
Route: /marketing/comments
Top: platform filter tabs (All / YouTube / Instagram / Facebook / TikTok).
Below: a scrollable list of comment cards. Each card shows: platform icon,
commenter avatar + name, comment text, time ago, and an AI-suggested reply in
a shaded box with "Use" and "Edit" controls. An "Approve & Send" button posts
the reply. A "Flag" button marks the comment for follow-up. A reply-policy
badge shows whether auto-reply is allowed for that platform.

Marketing page 7 — Analytics
Route: /marketing/analytics
A dashboard. Top: a date range picker (From / To). KPI row: total views, total
likes, total comments, average engagement rate — each as a stat card with
delta vs. previous period. Main charts: a line chart of views and likes over
time (two series), a bar chart of posts by format, and a "Hooks leaderboard"
table ranking hook open/engagement rates. All charts are inline SVG. A
"Group by" dropdown lets you break the line chart by product or format.

Marketing page 8 — Catalog
Route: /marketing/catalog
A product table: columns Image, SKU, Name, Category, Price, Actions (edit /
delete). A search box and category filter at top. An "Add product" button
opens a modal with fields: image upload dropzone, SKU, name, category
(select), price, and a rich-text description. Editing a row reopens the modal
prefilled. Deleting asks for confirmation.

DOMAIN: HR
The HR domain manages the hiring lifecycle — posting jobs, screening CVs,
moving candidates through interview stages, scheduling interviews,
communicating with candidates, and reporting on hiring performance.
Sidebar links (in order): Jobs, Pipeline, Candidates, Schedule, Comms,
Analytics.

HR page 1 — Jobs
Route: /hr/jobs
A job table: columns Title, Department, Status (Draft/Open/Closed), Posted
date, Actions (edit / delete / post). "New job" button opens a JD editor —
a markdown editor on the left with a live preview on the right, plus fields
for title, department, and location. A "Post to boards" console: a list of
board checkboxes (LinkedIn, TopDev, VietnamWorks, ITviec, own careers page),
a generated board-text preview that adapts to each board's length limit, and
a "Post" button.

HR page 2 — Pipeline (Kanban, per job)
Route: /hr/pipeline/:jobId
A Kanban with columns: Applied, Screened, Interview, Offer, Hired, Rejected.
Each card shows: candidate name, avatar, applied role, source (where they
applied from), a CV-fit score (0–100) as a small bar, and stage-move arrows.
Drag between columns to change stage. Clicking a card routes to the
Candidates page focused on that candidate. A job selector at top switches
between open jobs.

HR page 3 — Candidates
Route: /hr/candidates
Left: a searchable candidate list (name search + role filter). Each list item
shows name, avatar, current stage badge, and score. Selecting a candidate
loads the right panel: a CV viewer (embed a PDF in an <iframe>), a score
breakdown showing sub-scores per JD requirement as horizontal bars, contact
details, and a "Compare" checkbox. Selecting two or more candidates and
clicking "Compare" opens a side-by-side table of their sub-scores.

HR page 4 — Schedule
Route: /hr/schedule
Top: an event-type selector (e.g. "Initial screen", "Technical interview",
"Final interview"). Left: a Cal.com embed area (an <iframe> placeholder) for
the interviewer's calendar. Right: a slot picker — a list of available time
slots for the chosen date, each with a "Book" button. Booking opens a small
form: candidate selector (searchable), confirm. A "Send invite" button emails
the candidate a confirmation. A list of upcoming booked interviews appears
below.

HR page 5 — Comms
Route: /hr/comms
A two-pane messaging view. Left: a candidate list with a channel filter
(All / Zalo / Telegram / Email) and a search. Selecting a candidate loads the
right pane: a thread view of all messages with that candidate, each bubble
showing channel icon, direction (in/out), text, and timestamp. Bottom: a
message composer with a template picker dropdown (e.g. "Interview reminder",
"Offer letter", "Rejection") that fills the textarea, a channel selector, and
a "Send" button. A small log line confirms each send was recorded.

HR page 6 — Analytics
Route: /hr/analytics
A dashboard. Top: date range picker. KPI row: total applications, total
hires, time-to-hire (avg days), offer acceptance rate. Charts: a line chart
of applications over time, a funnel conversion chart (Applied → Screened →
Interview → Offer → Hired) shown as horizontal bars with percentages, and a
"Source effectiveness" bar chart ranking recruitment sources by hire count.
A "Group by job" toggle breaks the funnel per job.

DESIGN RULES (apply to every page)
- Use Tailwind CSS utility classes only; no custom CSS files.
- shadcn-style primitives: rounded-lg borders, subtle shadows, slate/zinc
  neutrals, one accent color (indigo) for primary actions and active states.
- Spacing scale: 4 / 8 / 12 / 16 / 24 / 32px. Max content width 1280px.
- Typography: one sans-serif (system-ui / Inter), two weights (400, 600).
- Iconography: simple inline SVG icons (heroicons style), 20px in nav.
- Tables: sticky header row, zebra rows off, hover highlight, 44px row height.
- Forms: labels above inputs, 8px gap, inputs 40px tall, focus ring indigo.
- Modals: centered, 480px wide, backdrop blur, Esc to close, click-outside to
  close.
- Empty states: a short headline + one-line description + a primary action.
- Loading states: skeleton rows with pulse animation.
- All interactive elements must be keyboard reachable.
- Charts are inline SVG with axis labels and a legend; no external chart lib
  in the artifact.
- Color: light mode background #ffffff / #f8fafc; dark mode #0f172a / #1e293b.
- No lorem ipsum — use realistic sample data that matches each page's domain
  (real product names, realistic candidate names, realistic comment text).
- Seperate the demo into multiple not mono-file.
```

### Run the master prompt

```bash
# Generate the full app shell + marketing domain as one artifact
od plugin apply example-dashboard \
  --input brief="$(cat ./.docs/phase-4-opendesign-prompt.md | sed -n '/^```text$/,/^```$/p' | sed '1d;$d')" \
  --output ./.design-output/office-agent-app.html
```

Or paste the prompt block directly into the Open Design web UI chat and pick
`example-dashboard` (or `example-flowai-live-dashboard-template` for a denser
admin layout) as the skill.

To render one page at a time, copy the single page's section out of the master
prompt (e.g. just the "Marketing page 1 — Pipeline" block) and prepend:

```text
Using the Office Agent design system (indigo accent, slate neutrals, shadcn
primitives, Tailwind v4), generate the following page as a self-contained
responsive HTML artifact. Include the app shell (top bar with Marketing|HR
switcher, left sidebar with the domain's nav). Page:
```

---

## Handoff checklist

After generating artifacts under `.design-output/`:

1. Open each `.html` artifact in a browser.
2. Verify layout at 1280px, 1440px, and mobile widths.
3. Screenshot the result and attach to the Phase 4 ticket.
4. Extract Tailwind classes, chart markup, and component structure.
5. Port into the React app:
   - `agent-core/web/src/pages/marketing/`
   - `agent-core/web/src/pages/hr/`
6. Replace static sample data with calls from `agent-core/web/src/lib/api.ts`.
7. Run `cd agent-core/web && npm run build` and fix TypeScript errors.

---

## Apply a design system

If you want all 14 pages to share one brand system:

```bash
# List shipped systems
od plugin list --tag design-system

# Examples: linear-app, stripe, vercel, notion, default
od plugin info linear-app

# Re-style generated artifacts or existing codebase
od plugin apply od-code-migration \
  --input repo="." \
  --input design-system="linear-app"
```

---

## Verify everything works

```bash
od --version
od mcp install opencode --print
od plugin list --scenario marketing --json | jq '.[].name'
od plugin apply example-dashboard --input brief="test"
```

Expected results:
- `od --version` prints a version number.
- `od mcp install opencode --print` shows valid MCP JSON.
- `od plugin list` returns plugin names.
- `od plugin apply` creates an HTML artifact without errors.

---

## Links

- Open Design: https://open-design.ai/
- Downloads: https://open-design.ai/download/
- Quickstart: https://open-design.ai/quickstart/
- GitHub: https://github.com/nexu-io/open-design
- OpenCode design guide: https://open-design.ai/agents/opencode-design/
