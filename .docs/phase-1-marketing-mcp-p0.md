# Phase 1 — Marketing MCP P0 (Rust)

**Goal:** 3 local Rust MCP servers (`mcp-catalog`, `mcp-video-edit`,
`mcp-ledger`) fully implemented and responding to `hermes mcp call`.
**Depends on:** Phase 0.
**Duration:** ~1 week.
**Parallelizable with:** Phase 1b (HR MCP P0) — different crates, no conflict.

Read [`00-overview.md`](./00-overview.md) first for workspace structure and
conventions.

---

## 1.1 `mcp-catalog` — product catalog (Rust + rusqlite)

**Files:**
- `optional-mcps/mcp-catalog/src/main.rs` — rmcp server entry
- `optional-mcps/mcp-catalog/src/tools.rs` — 3 tool handlers
- `optional-mcps/mcp-catalog/src/db.rs` — SQLite connection pool
- `optional-mcps/mcp-catalog/manifest.toml` — registration

**Tools (exact signatures — JSON schema in rmcp annotations):**

| Tool | Params | Returns | SQL |
| ---- | ------ | ------- | --- |
| `list_products` | `category?: string`, `limit?: int` | `[{sku,name,category,price_vnd}]` | `SELECT sku,name,category,price_vnd FROM products [WHERE category=?] LIMIT ?` |
| `get_product_specs` | `sku: string` | `{sku,name,specs_json,price_vnd,tags,image_paths}` | `SELECT * FROM products WHERE sku=?` |
| `search_catalog` | `query: string`, `limit?: int` | `[{sku,name,category,price_vnd,tags}]` | `SELECT sku,name,category,price_vnd,tags FROM products WHERE name LIKE ? OR tags LIKE ? OR category LIKE ? LIMIT ?` |
| `health` | — | `{"ok":true,"name":"mcp-catalog"}` | — |

**DB path:** `$CATALOG_DB` env var, default `~/.hermes/data/catalog.db`.
On startup: if DB file missing, run `schema.sql` + `seed.sql`.

**`manifest.toml`:**
```toml
name = "mcp-catalog"
command = "./target/release/mcp-catalog"
transport = "stdio"
env = { CATALOG_DB = "~/.hermes/data/catalog.db" }
scopes = ["catalog:read"]
```

**Verify:**
- `cargo test -p mcp-catalog` — unit tests for each tool against in-memory
  `:memory:` SQLite with seed data.
- `cargo run -p mcp-catalog` then pipe JSON-RPC → `health` returns ok.
- `hermes mcp call mcp-catalog list_products '{}'` → returns 9 products.
- `hermes mcp call mcp-catalog get_product_specs '{"sku":"MA5"}'` → returns
  MA5 row.
- `hermes mcp call mcp-catalog search_catalog '{"query":"mic"}'` → returns
  mic-related products.

## 1.2 `mcp-video-edit` — FFmpeg pipeline (Rust + subprocess)

**Files:**
- `optional-mcps/mcp-video-edit/src/main.rs`
- `optional-mcps/mcp-video-edit/src/tools.rs` — 7 tool handlers
- `optional-mcps/mcp-video-edit/src/ffmpeg.rs` — subprocess wrapper + logging
- `optional-mcps/mcp-video-edit/manifest.toml`

**External deps (system):** `ffmpeg`, `ffprobe` on PATH. For captions:
`whisper.cpp` binary at `$WHISPER_BIN` (default `whisper-cli`), model at
`$WHISPER_MODEL` (default `~/.hermes/models/ggml-base.bin`).

**Tools:**

| Tool | Params | Returns | FFmpeg op |
| ---- | ------ | ------- | --------- |
| `cut_by_shotlist` | `input: path`, `shots: [{start_sec,end_sec,label}]` | `{output_path, segment_count}` | `ffmpeg -i input -ss start -to end -c copy segment_n.mp4` per shot, then concat |
| `burn_captions` | `input: path`, `captions: [{start_sec,end_sec,text}]`, `style?: {font,size,color,bg}` | `{output_path}` | burn .ass subtitles via `ffmpeg -vf ass=captions.ass` |
| `overlay_text` | `input: path`, `overlays: [{start_sec,end_sec,text,position}]` | `{output_path}` | `drawtext` filter |
| `add_music` | `input: path`, `music: path`, `level_db?: float`, `duck?: bool` | `{output_path}` | `amix` + sidechain ducking |
| `encode_916` | `input: path`, `quality?: string` | `{output_path, width, height}` | scale+crop to 1080x1920, H.264, AAC |
| `concat` | `inputs: [path]`, `transition?: string` | `{output_path}` | concat demuxer |
| `extract_audio` | `input: path`, `format?: string` | `{output_path}` | `-vn -acodec` |
| `health` | — | `{"ok":true,"name":"mcp-video-edit"}` | — |

**Rules:**
- All output files written to `$EDIT_WORKSPACE` (default `~/.hermes/work/`).
- Each tool returns absolute `output_path`.
- `cut_by_shotlist` validates shots don't overlap and sum ≤ input duration
  (use `ffprobe` to get input duration).
- `burn_captions` without explicit captions → auto-transcribe via whisper.cpp
  first, then burn.

**Verify:**
- `which ffmpeg ffprobe` both found.
- `cargo test -p mcp-video-edit` — mock FFmpeg with a test binary, assert
  arg construction.
- Prepare a 10s test mp4 at `tests/fixtures/test.mp4`.
- `hermes mcp call mcp-video-edit encode_916 '{"input":"tests/fixtures/test.mp4"}'`
  → output is 1080x1920 (verify with `ffprobe`).
- `hermes mcp call mcp-video-edit cut_by_shotlist '{"input":"tests/fixtures/test.mp4","shots":[{"start_sec":0,"end_sec":3,"label":"a"},{"start_sec":3,"end_sec":10,"label":"b"}]}'`
  → output exists, 2 segments.

## 1.3 `mcp-ledger` — performance ledger (Rust + rusqlite)

**Files:**
- `optional-mcps/mcp-ledger/src/main.rs`
- `optional-mcps/mcp-ledger/src/tools.rs` — 4 tool handlers
- `optional-mcps/mcp-ledger/src/db.rs`
- `optional-mcps/mcp-ledger/manifest.toml`

**Tools:**

| Tool | Params | Returns | SQL |
| ---- | ------ | ------- | --- |
| `record_post` | `piece_id, product_sku, format, platform, platform_post_id?, caption?, hook_text?` | `{post_id}` | INSERT into posts |
| `get_performance` | `post_id: int` or `piece_id: string` | `{views,likes,comments,shares,watch_time_sec}` | JOIN posts+metrics |
| `query_what_worked` | `group_by: "format"\|"platform"\|"product_sku"`, `date_from?`, `date_to?` | `[{key, avg_views, avg_likes, avg_retention, count}]` | GROUP BY aggregation |
| `get_hooks_leaderboard` | `limit?: int`, `format?: string` | `[{hook_text, avg_retention, uses, last_used_at}]` | SELECT FROM hooks ORDER BY avg_retention |
| `health` | — | `{"ok":true,"name":"mcp-ledger"}` | — |

**DB path:** `$LEDGER_DB` default `~/.hermes/data/ledger.db`.

**Verify:**
- `cargo test -p mcp-ledger` — in-memory DB, insert 5 posts + metrics, assert
  `query_what_worked` groupings.
- `hermes mcp call mcp-ledger record_post '{"piece_id":"p1","product_sku":"MA5","format":"demo","platform":"youtube"}'`
  → returns `{post_id: 1}`.
- `hermes mcp call mcp-ledger get_hooks_leaderboard '{"limit":5}'` → returns
  array (empty or seeded).

## 1.4 Register all 3 + integration smoke test

**Do:**
1. Write `manifest.toml` for each of the 3 MCPs.
2. `hermes mcp register optional-mcps/mcp-catalog/manifest.toml` (repeat for
   each).
3. `hermes mcp list` → shows all 3.
4. Run `health` on each: `hermes mcp call <name> health '{}'`.

**Verify:**
- All 3 appear in `hermes mcp list` with status `connected`.
- All 3 `health` calls return `{"ok":true,...}`.
- Write `optional-mcps/SMOKE.md` recording the 3 successful call transcripts.

---

## Phase 1 exit criteria

- [ ] `cargo build --workspace --release` — 3 binaries in `target/release/`
- [ ] `cargo test --workspace` passes
- [ ] 3 `manifest.toml` registered, `hermes mcp list` shows all 3
- [ ] `SMOKE.md` has successful transcripts for all 3 `health` + at least
      one non-trivial tool each
