# Phase 0 — Bootstrap & parallel lead-times

**Goal:** Hermes base running, workspace scaffolded, long-lead app reviews
started, SQLite schemas frozen, Zalo plugin live.
**Depends on:** nothing.
**Duration:** ~2-3 active days + weeks of waiting (tracked in LEADTIMES.md).
**Parallelizable with:** nothing — this is the foundation.

Read [`00-overview.md`](./00-overview.md) first for workspace structure and
conventions.

---

## 0.1 Bring in agent-core base

**Do:**
1. `git remote add agent-core https://github.com/LynxLabVN/agent-core.git`
2. `git fetch agent-core`
3. `git subtree add --prefix=agent-core agent-core main --squash`
4. Record the pinned commit hash in `LEADTIMES.md` under "base pin".

**Verify:**
- `cd agent-core && python run_agent.py --help` exits 0.
- `hermes --version` prints a version string.
- `cd agent-core/web && npm install && npm run build` succeeds and writes to
  `agent-core/hermes_cli/web_dist/`.

## 0.2 Create workspace scaffold

**Do:**
1. `mkdir -p optional-mcps skills/marketing/marketing-pipeline skills/hr/recruitment`
2. Write `optional-mcps/Cargo.toml` (workspace root — see `00-overview.md`).
3. Write `optional-mcps/rust-toolchain.toml` pinning stable.
4. Write `LEADTIMES.md` with a table for the 4 app reviews (see below).

**`LEADTIMES.md` skeleton:**
```markdown
# Lead-time tracker

| Gate | Started | Status | Blocks phase | ETA |
| ---- | ------- | ------ | ------------ | --- |
| Meta app review | | | Phase 3 (mcp-social-meta) | |
| Zalo OA verification | | | Phase 3 (mcp-zalo-oa) | |
| TikTok Content Posting API | | | Phase 3 (mcp-social-tiktok) | |
| YouTube Data API OAuth consent | | | Phase 3 (mcp-social-youtube) | |
| base pin (agent-core commit) | | | — | n/a |

## social-media skill map
(to fill in 0.7)

## company source → MCP map
(to fill in 0.7)
```

**Verify:** `cargo build --workspace` compiles (empty members OK if Cargo
errors, add one stub crate first — see 0.3).

## 0.3 Rust MCP crate stub (do once, clone for each MCP)

Create `optional-mcps/mcp-catalog/` as the template stub:

**`optional-mcps/mcp-catalog/Cargo.toml`:**
```toml
[package]
name = "mcp-catalog"
version.workspace = true
edition.workspace = true
license.workspace = true

[[bin]]
name = "mcp-catalog"
path = "src/main.rs"

[dependencies]
rmcp.workspace = true
tokio.workspace = true
serde.workspace = true
serde_json.workspace = true
rusqlite.workspace = true
anyhow.workspace = true
tracing.workspace = true
tracing-subscriber.workspace = true
```

**`optional-mcps/mcp-catalog/src/main.rs`** — minimal rmcp server that
registers one `health` tool. (Full implementation in Phase 1.)

**Verify:** `cargo build -p mcp-catalog` succeeds. `cargo run -p mcp-catalog`
starts an stdio server that answers `health`.

## 0.4 Start long-lead app reviews (parallel, weeks)

**Do (external, manual — kick off and record in LEADTIMES.md):**
1. **Meta:** `developers.facebook.com` → create app → add Instagram Graph API
   + Facebook Graph API + Pages → submit for app review with
   `instagram_content_publish` + `pages_manage_posts` + `pages_read_engagement`.
2. **Zalo OA:** `zalo.cloud` → register Official Account → submit business
   docs → get `OA_ACCESS_TOKEN` (long-lived) → prepare webhook URL
   `https://<host>/zalo-oa/webhook` + `ZALO_OA_WEBHOOK_SECRET`.
3. **TikTok:** `developers.tiktok.com` → create app → apply for Content
   Posting API scope.
4. **YouTube:** `console.cloud.google.com` → create project → enable YouTube
   Data API v3 → configure OAuth consent screen → create OAuth2 credentials
   (desktop app type) → save client_id/secret for Hermes env vault.

**Verify:** 4 rows in LEADTIMES.md have "Started" date filled. Do not block
on completion — proceed to 0.5 onward while these cook.

## 0.5 Install hermes-zalo-plugin

**Do:**
```bash
npm install -g hermes-zalo-plugin
hermes-zalo-plugin setup        # scan QR with SECONDARY Zalo account
hermes gateway setup            # choose "Zalo", pick test candidate thread
hermes gateway                  # start relaying
```

Set in Hermes env (`~/.hermes/config` or vault):
```
ZALO_ALLOWED_THREADS=<test-thread-id>
ZALO_GROUP_MODE=mention
ZALO_ALLOW_DESTRUCTIVE=false
```

**Verify:**
- Send a test message to a known thread via `POST /send`:
  `curl -X POST localhost:<port>/send -d '{"threadId":"<id>","threadType":"user","text":"test"}'`
  → message appears on the recipient's Zalo.
- Reply from recipient → inbound SSE event logged by gateway.
- Kill and restart `hermes gateway` → reconnects without re-QR.

## 0.6 Freeze SQLite schemas

Write three schema files. These are the contract for Phase 1 / 1b.

**`optional-mcps/mcp-catalog/schema.sql`:**
```sql
CREATE TABLE IF NOT EXISTS products (
    sku         TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    category    TEXT NOT NULL,          -- audio | visual | accessory
    specs_json  TEXT NOT NULL,          -- JSON blob: {key, weight, dims, ...}
    price_vnd   INTEGER NOT NULL,
    tags        TEXT NOT NULL DEFAULT '', -- comma-separated
    image_paths TEXT NOT NULL DEFAULT '', -- comma-separated file paths
    created_at  TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at  TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_products_category ON products(category);
CREATE INDEX IF NOT EXISTS idx_products_tags ON products(tags);
-- Seed rows: MA5, A14, AD35, A8, GX200, P011, bao đàn, UHF, mic đeo tai
```

**`optional-mcps/mcp-ledger/schema.sql`:**
```sql
CREATE TABLE IF NOT EXISTS posts (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    piece_id        TEXT NOT NULL,           -- links to pipeline piece
    product_sku     TEXT NOT NULL,
    format          TEXT NOT NULL,           -- short_form | ugc | demo | ...
    platform        TEXT NOT NULL,           -- youtube | meta | tiktok
    platform_post_id TEXT,                   -- returned by platform
    caption         TEXT,
    hook_text       TEXT,
    posted_at       TEXT NOT NULL DEFAULT (datetime('now')),
    status          TEXT NOT NULL DEFAULT 'published'
);
CREATE INDEX IF NOT EXISTS idx_posts_product ON posts(product_sku);
CREATE INDEX IF NOT EXISTS idx_posts_platform ON posts(platform);

CREATE TABLE IF NOT EXISTS metrics (
    post_id         INTEGER PRIMARY KEY,     -- FK -> posts.id
    views           INTEGER DEFAULT 0,
    likes           INTEGER DEFAULT 0,
    comments        INTEGER DEFAULT 0,
    shares          INTEGER DEFAULT 0,
    watch_time_sec  INTEGER DEFAULT 0,
    pulled_at       TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (post_id) REFERENCES posts(id)
);

CREATE TABLE IF NOT EXISTS hooks (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    hook_text       TEXT NOT NULL,
    product_sku     TEXT,
    format          TEXT,
    avg_retention   REAL DEFAULT 0,
    uses            INTEGER DEFAULT 0,
    last_used_at    TEXT
);
CREATE INDEX IF NOT EXISTS idx_hooks_retention ON hooks(avg_retention DESC);
```

**`optional-mcps/mcp-hr-data/schema.sql`:**
```sql
CREATE TABLE IF NOT EXISTS jobs (
    id              TEXT PRIMARY KEY,        -- uuid
    title           TEXT NOT NULL,
    dept            TEXT NOT NULL,
    jd_markdown     TEXT NOT NULL,
    skills_required TEXT NOT NULL,           -- comma-separated
    skills_nice     TEXT NOT NULL DEFAULT '',
    exp_level       TEXT NOT NULL,           -- junior | mid | senior | lead
    salary_min_vnd  INTEGER,
    salary_max_vnd  INTEGER,
    location        TEXT NOT NULL,
    benefits        TEXT NOT NULL DEFAULT '',
    status          TEXT NOT NULL DEFAULT 'draft', -- draft|review|open|closed
    created_at      TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at      TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS candidates (
    id              TEXT PRIMARY KEY,        -- uuid
    full_name       TEXT NOT NULL,
    email           TEXT,
    phone           TEXT,
    zalo_uid        TEXT,
    cv_file_path    TEXT,
    portfolio_urls  TEXT NOT NULL DEFAULT '', -- newline-separated
    parsed_json     TEXT,                    -- output of mcp-cv-screen.parse_cv
    created_at      TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_candidates_phone ON candidates(phone);
CREATE INDEX IF NOT EXISTS idx_candidates_zalo ON candidates(zalo_uid);

CREATE TABLE IF NOT EXISTS applications (
    id              TEXT PRIMARY KEY,        -- uuid
    job_id          TEXT NOT NULL,
    candidate_id    TEXT NOT NULL,
    stage           TEXT NOT NULL DEFAULT 'applied',
        -- applied|screened|shortlist|interview|offer|hired|rejected
    cv_score        REAL,                    -- 0-100 from mcp-cv-screen
    score_breakdown TEXT,                    -- JSON
    source          TEXT,                    -- zalo_oa|email|manual|board
    applied_at      TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at      TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (job_id) REFERENCES jobs(id),
    FOREIGN KEY (candidate_id) REFERENCES candidates(id)
);
CREATE INDEX IF NOT EXISTS idx_apps_job ON applications(job_id);
CREATE INDEX IF NOT EXISTS idx_apps_stage ON applications(stage);

CREATE TABLE IF NOT EXISTS interviews (
    id              TEXT PRIMARY KEY,
    application_id  TEXT NOT NULL,
    round           TEXT NOT NULL,           -- screen|technical|culture|final
    scheduled_at    TEXT NOT NULL,
    duration_min    INTEGER NOT NULL DEFAULT 60,
    interviewer     TEXT NOT NULL,
    calcom_booking_id TEXT,
    notes_markdown  TEXT,
    decision        TEXT,                    -- pass|fail|hold|null
    created_at      TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (application_id) REFERENCES applications(id)
);

CREATE TABLE IF NOT EXISTS comms_log (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    candidate_id    TEXT,
    application_id  TEXT,
    channel         TEXT NOT NULL,           -- zalo_personal|zalo_oa|telegram|email
    direction       TEXT NOT NULL,           -- inbound|outbound
    message_text    TEXT NOT NULL,
    template_used   TEXT,
    sent_by         TEXT NOT NULL DEFAULT 'agent', -- agent|human
    ts              TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (candidate_id) REFERENCES candidates(id),
    FOREIGN KEY (application_id) REFERENCES applications(id)
);
CREATE INDEX IF NOT EXISTS idx_comms_candidate ON comms_log(candidate_id);
CREATE INDEX IF NOT EXISTS idx_comms_channel ON comms_log(channel);
```

**Verify:**
- `sqlite3 /tmp/test_catalog.db < optional-mcps/mcp-catalog/schema.sql` exits 0.
- `sqlite3 /tmp/test_ledger.db < optional-mcps/mcp-ledger/schema.sql` exits 0.
- `sqlite3 /tmp/test_hr.db < optional-mcps/mcp-hr-data/schema.sql` exits 0.
- Seed 9 product rows into `mcp-catalog/schema.sql` seed section (or a
  separate `seed.sql`).

## 0.7 Inspect existing skills + map company source

**Do:**
1. Read `agent-core/skills/social-media/SKILL.md` — list existing commands and
   tool calls. Write a one-paragraph summary to `LEADTIMES.md` under
   "social-media skill map".
2. If company has existing auto-reply / post / analytics Python source (user
   to paste into `agent-core/company-src/`), inventory what each script does
   and which MCP it maps to. Write mapping to `LEADTIMES.md` under
   "company source → MCP map".

**Verify:** LEADTIMES.md has both summary sections filled.

---

## Phase 0 exit criteria

- [ ] `python agent-core/run_agent.py --help` works
- [ ] `cargo build --workspace` compiles (mcp-catalog stub)
- [ ] LEADTIMES.md: 4 app reviews started + base pin recorded + skill map + source map
- [ ] Zalo plugin sends + receives a test message
- [ ] 3 schema.sql files pass `sqlite3 < file` with seed data
- [ ] `hermes mcp list` shows `mcp-catalog` registered (even if stub)
