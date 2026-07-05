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
