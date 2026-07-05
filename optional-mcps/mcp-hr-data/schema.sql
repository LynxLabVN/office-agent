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
    score_override  REAL,                    -- human override of cv_score
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
