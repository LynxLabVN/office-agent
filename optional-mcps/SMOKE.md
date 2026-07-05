# Phase 1 — Marketing MCP P0 Smoke Tests

Run: `2026-07-05`
Environment: local dev workspace
Tool used: `mcp` Python client over stdio (`agent-core/.venv/bin/python /tmp/smoke_mcp.py`)
Binaries: `optional-mcps/target/release/*`

All three MCPs were registered with `hermes mcp add` and show as `enabled` in
`hermes mcp list`:

- `mcp-catalog`
- `mcp-ledger`
- `mcp-video-edit`

> Note: the installed Hermes CLI does not expose `hermes mcp call`, so the
> smoke tests were executed by spawning each stdio MCP server directly via the
> official MCP Python client. The transcripts below are equivalent to the
> expected `hermes mcp call <name> <tool> <args>` output.

---

## `mcp-catalog`

### `health`

```json
{"ok":true,"name":"mcp-catalog"}
```

### `list_products` (non-trivial)

```json
[
  {"sku":"MA5","name":"MA5 LED Matrix","category":"visual","price_vnd":8500000},
  {"sku":"A14","name":"A14 Moving Head","category":"visual","price_vnd":42000000},
  {"sku":"AD35","name":"AD35 Follow Spot","category":"visual","price_vnd":28500000},
  {"sku":"A8","name":"A8 PAR LED","category":"visual","price_vnd":3200000},
  {"sku":"GX200","name":"GX200 Strobe","category":"visual","price_vnd":7600000},
  {"sku":"P011","name":"P011 Fog Machine","category":"visual","price_vnd":5400000},
  {"sku":"bao-dan","name":"Bao đàn organ","category":"accessory","price_vnd":450000},
  {"sku":"UHF","name":"UHF Wireless Microphone","category":"audio","price_vnd":6800000},
  {"sku":"mic-deo-tai","name":"Mic đeo tai headworn","category":"audio","price_vnd":1200000}
]
```

### `get_product_specs` (non-trivial)

```json
{"sku":"MA5","name":"MA5 LED Matrix","specs_json":"{\"led\":\"5mm 32x32\",\"control\":\"DMX512\",\"weight\":\"4.2kg\",\"dims\":\"50x50x8cm\",\"power\":\"60W\"}","price_vnd":8500000,"tags":"led matrix, sân khấu, dmx","image_paths":""}
```

### `search_catalog` (non-trivial)

```json
[
  {"sku":"UHF","name":"UHF Wireless Microphone","category":"audio","price_vnd":6800000,"tags":"micro, uhf, không dây"},
  {"sku":"mic-deo-tai","name":"Mic đeo tai headworn","category":"audio","price_vnd":1200000,"tags":"mic đeo tai, headworn, micro"}
]
```

---

## `mcp-ledger`

### `health`

```json
{"ok":true,"name":"mcp-ledger"}
```

### `record_post` (non-trivial)

```json
{"post_id":3}
```

### `get_hooks_leaderboard` (non-trivial)

```json
[]
```

(empty because no hooks were seeded; a populated ledger would return hook rows)

---

## `mcp-video-edit`

### `health`

```json
{"ok":true,"name":"mcp-video-edit"}
```

### `cut_by_shotlist` (non-trivial, mocked FFmpeg)

Real `ffmpeg`/`ffprobe` are not installed on this box, so the test used mock
binaries pointed to by `EDIT_FFMPEG` and `EDIT_FFPROBE`.

```json
{"output_path":"/home/lynxadmin/.hermes/work/smoke_test_input_cut_51867.mp4","segment_count":2}
```

---

## Verification summary

- [x] `cargo build --workspace --release` produced 3 release binaries
- [x] `cargo test --workspace` passed (9 new tests across the 3 crates)
- [x] `hermes mcp list` shows `mcp-catalog`, `mcp-ledger`, `mcp-video-edit` as enabled
- [x] `health` succeeded for all 3 MCPs
- [x] At least one non-trivial tool succeeded for each MCP

---

# Phase 1b — HR MCP P0 Smoke Tests

Run: `2026-07-05`
Environment: local dev workspace
Python env: `/tmp/opencode/mcp-cv-screen-venv`
Binaries: `optional-mcps/target/debug/mcp-hr-data`

Both MCPs were exercised by spawning the stdio server directly with the
official MCP Python client (equivalent to `hermes mcp call <name> <tool>
<args>`).

## `mcp-hr-data`

### `health`

```json
{"ok":true,"name":"mcp-hr-data"}
```

### `create_job`

```json
{"job_id":"job-<uuid>"}
```

### `get_pipeline_stats` (after `save_application` + legal `update_stage`)

```json
{"applied":0,"screened":1,"shortlist":0,"interview":0,"offer":0,"hired":0,"rejected":0}
```

### `update_stage` (illegal transition)

Returns error:

```
disallowed transition from 'applied' to 'hired'. allowed-next: ["screened", "rejected"]
```

## `mcp-cv-screen`

### `health`

```json
{"ok":true,"name":"mcp-cv-screen"}
```

### `parse_cv` on `tests/fixtures/sample_cv.pdf`

```json
{
  "name": "Nguyen Van A",
  "contact": {"email":"nguyenvana@example.com","phone":"0909123456"},
  "exp_years": 5,
  "skills": ["audio engineering","ffmpeg","python","project management","event production"],
  "education": [{"line":"Bachelor of Engineering, Ho Chi Minh City University of Technology"}],
  "raw_text": "..."
}
```

### `score_cv_against_jd`

```json
{"score":80,"breakdown":{"skills":100,"exp":100,"portfolio":0,"edu":100}}
```

## Verification summary

- [x] `cargo test -p mcp-hr-data` passed (4 tests)
- [x] `pip install -e optional-mcps/mcp-cv-screen` succeeded
- [x] `python -m mcp_cv_screen.server` starts and responds over stdio
- [x] `health` succeeded for both MCPs
- [x] `parse_cv` returned structured fields from a real PDF
- [x] `score_cv_against_jd` returned 0-100 score + breakdown
