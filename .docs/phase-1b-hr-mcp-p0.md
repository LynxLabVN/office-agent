# Phase 1b — HR MCP P0 (Rust + Python)

**Goal:** `mcp-hr-data` (Rust) + `mcp-cv-screen` (Python) working end-to-end.
Zalo plugin verified with a real candidate reply simulation.
**Depends on:** Phase 0.
**Duration:** ~3-4 days.
**Parallelizable with:** Phase 1.

Read [`00-overview.md`](./00-overview.md) first for workspace structure and
conventions.

---

## 1b.1 `mcp-hr-data` — HR data layer (Rust + rusqlite)

**Files:**
- `optional-mcps/mcp-hr-data/src/main.rs`
- `optional-mcps/mcp-hr-data/src/tools.rs` — 9 tool handlers
- `optional-mcps/mcp-hr-data/src/db.rs`
- `optional-mcps/mcp-hr-data/manifest.toml`

**Tools:**

| Tool | Params | Returns |
| ---- | ------ | ------- |
| `create_job` | `title, dept, jd_markdown, skills_required[], skills_nice[]?, exp_level, salary_min_vnd?, salary_max_vnd?, location, benefits?` | `{job_id}` |
| `list_jobs` | `status?: string` | `[{id,title,dept,exp_level,status,created_at}]` |
| `get_job` | `job_id: string` | full job row |
| `save_application` | `job_id, candidate_id, source` | `{application_id}` |
| `list_applications` | `job_id: string`, `stage?: string` | `[{id,candidate_id,stage,cv_score,applied_at}]` |
| `get_candidate` | `candidate_id: string` | candidate row + parsed_json |
| `save_interview_note` | `interview_id, notes_markdown, decision` | `{ok:true}` |
| `update_stage` | `application_id, new_stage` | `{ok:true, previous_stage}` |
| `get_pipeline_stats` | `job_id: string` | `{applied,screened,shortlist,interview,offer,hired,rejected}` |
| `health` | — | `{"ok":true,"name":"mcp-hr-data"}` |

**Stage transition rules (enforced in `update_stage`):**
```
applied → screened → shortlist → interview → offer → hired
                 ↘ rejected    ↘ rejected    ↘ rejected
```
Any disallowed transition returns error with the allowed-next list.

**DB path:** `$HR_DB` default `~/.hermes/data/hr.db`.

**Verify:**
- `cargo test -p mcp-hr-data` — create job, save application, update stage
  (legal + illegal), get stats.
- `hermes mcp call mcp-hr-data create_job '{"title":"Audio Engineer","dept":"AV","jd_markdown":"...","skills_required":["audio","ffmpeg"],"exp_level":"mid","location":"HCM"}'`
  → returns `{job_id}`.
- `hermes mcp call mcp-hr-data get_pipeline_stats '{"job_id":"<id>"}'` →
  returns stage counts.

## 1b.2 `mcp-cv-screen` — CV/portfolio screening (Python/FastMCP)

**Why Python:** Whisper (speech-to-text for video portfolios) + PyMuPDF (PDF
parse) + Pillow (image OCR fallback) have no Rust equivalent without FFI pain.

**Files:**
- `optional-mcps/mcp-cv-screen/pyproject.toml`
- `optional-mcps/mcp-cv-screen/src/mcp_cv_screen/__init__.py`
- `optional-mcps/mcp-cv-screen/src/mcp_cv_screen/server.py` — FastMCP server
- `optional-mcps/mcp-cv-screen/src/mcp_cv_screen/parse.py` — PDF/image parse
- `optional-mcps/mcp-cv-screen/src/mcp_cv_screen/skills.py` — skill extraction
- `optional-mcps/mcp-cv-screen/src/mcp_cv_screen/score.py` — scoring rubric
- `optional-mcps/mcp-cv-screen/src/mcp_cv_screen/portfolio.py` — web/video
- `optional-mcps/mcp-cv-screen/manifest.toml`

**`pyproject.toml` deps:**
```toml
[project]
name = "mcp-cv-screen"
version = "0.1.0"
requires-python = ">=3.11"
dependencies = [
    "fastmcp>=0.1",
    "pymupdf>=1.24",
    "pillow>=10",
    "faster-whisper>=1.0",
    "httpx>=0.27",
    "anyio>=4",
]
```

**Tools:**

| Tool | Params | Returns | Impl |
| ---- | ------ | ------- | ---- |
| `parse_cv` | `file_path: string` | `{name, contact, exp_years, skills[], education, raw_text}` | PyMuPDF → text → regex/LLM extract |
| `extract_skills` | `text: string` | `{skills: [{name, normalized, confidence}]}` | keyword map + LLM normalization via Hermes agent |
| `score_cv_against_jd` | `cv_id: string`, `jd_id: string` | `{score: 0-100, breakdown: {skills: x, exp: x, portfolio: x, edu: x}}` | rubric: skills 40% + exp 30% + portfolio 20% + edu 10% |
| `analyze_portfolio` | `url_or_files: string[]` | `{screenshots[], text_extracted, transcript?}` | httpx for web, faster-whisper for video |
| `compare_candidates` | `cv_ids: string[]`, `jd_id: string` | `[{cv_id, name, score, rank}]` | call score_cv_against_jd per candidate, sort |
| `summarize_profile` | `cv_id: string` | `{summary: string (3 lines)}` | LLM via Hermes agent |
| `health` | — | `{"ok":true,"name":"mcp-cv-screen"}` | — |

**How LLM calls work:** `mcp-cv-screen` does NOT call the LLM API directly. It
returns structured prompts via tool results, and the Hermes agent does the
LLM call. Tools that say "LLM via Hermes agent" = tool returns the extracted
text + a suggested prompt; the agent loop does the actual inference. This
keeps the MCP simple and lets prompt caching work.

**Run:** `python -m mcp_cv_screen.server` (stdio).

**`manifest.toml`:**
```toml
name = "mcp-cv-screen"
command = "python"
args = ["-m", "mcp_cv_screen.server"]
transport = "stdio"
env = { WHISPER_MODEL = "base", CV_CACHE_DIR = "~/.hermes/data/cv_cache" }
scopes = ["hr:read", "hr:write"]
```

**Verify:**
- `pip install -e optional-mcps/mcp-cv-screen` succeeds.
- `python -m mcp_cv_screen.server` starts stdio server.
- `hermes mcp call mcp-cv-screen health '{}'` → ok.
- Put a sample PDF at `tests/fixtures/sample_cv.pdf`.
- `hermes mcp call mcp-cv-screen parse_cv '{"file_path":"tests/fixtures/sample_cv.pdf"}'`
  → returns structured fields.
- `hermes mcp call mcp-cv-screen score_cv_against_jd '{"cv_id":"...","jd_id":"..."}'`
  → returns 0-100 score + breakdown.

## 1b.3 Verify Zalo plugin end-to-end (HR scenario)

**Do:**
1. From Phase 0.5, the plugin is installed + QR logged in.
2. Simulate candidate inbound: send a message FROM a second phone TO the
   recruiter Zalo thread → gateway SSE event fires → check Hermes logs show
   inbound.
3. Simulate recruiter outbound: `hermes mcp call` is not needed — use the
   existing `messages_send` tool: `hermes mcp call hermes messages_send
   '{"target":"zalo:<threadId>","text":"Cảm ơn bạn đã ứng tuyển"}'`
   → message arrives on candidate phone.
4. Test attachment: `POST /send-attachment` with a PDF → candidate receives
   file.

**Verify:**
- Both directions work (inbound SSE + outbound REST).
- Attachment (PDF) delivers correctly.
- `comms_log` — not yet wired (that's Phase 3), just confirm the transport.

---

## Phase 1b exit criteria

- [ ] `cargo test -p mcp-hr-data` passes
- [ ] `pip install -e optional-mcps/mcp-cv-screen` + server starts + `health`
      ok
- [ ] `parse_cv` on a real PDF returns structured fields
- [ ] Zalo plugin: inbound + outbound + attachment verified with real phones
- [ ] 2 `manifest.toml` registered, `hermes mcp list` shows both
