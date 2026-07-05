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
    hook_text       TEXT NOT NULL UNIQUE,
    product_sku     TEXT,
    format          TEXT,
    avg_retention   REAL DEFAULT 0,
    uses            INTEGER DEFAULT 0,
    last_used_at    TEXT
);
CREATE INDEX IF NOT EXISTS idx_hooks_retention ON hooks(avg_retention DESC);
