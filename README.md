# Office Agent

A Marketing + HR agent built as a slim overlay on the
[Hermes](https://github.com/NousResearch/hermes-agent) base (fork
`LynxLabVN/agent-core`). 11 MCP servers (10 Rust + 1 Python), a marketing +
HR skill set, a React dashboard, and a single offline Windows installer.

- **CLI command:** `eir` (a shim over the Hermes CLI — same entry points,
  friendlier name)
- **Dashboard:** `eir dashboard` → http://127.0.0.1:9119/
- **MCPs from the CLI:** `eir mcp call <server> <tool> [json-args]`

## Repo layout

```
office-agent/
├── agent-core/            Hermes base + office-agent delta (see "Slim repo" below)
│   ├── hermes_cli/        base + modified files (web_server.py, main.py, config.py,
│   │                     mcp_config.py [+cmd_mcp_call], subcommands/mcp.py, …)
│   ├── skills/{hr,marketing,reply-policy,audit,quota_budgeter}/   office-agent skills
│   └── web/               React dashboard (builds → hermes_cli/web_dist/)
├── optional-mcps/         10 Rust MCP crates + mcp-cv-screen (Python)
│   └── target/release/   built MCP .exe binaries
├── tools/
│   ├── provision_base.py  restore/overlay the Hermes base (slim-repo machinery)
│   ├── base_files.txt     5927 vanilla-Hermes files (restored, not tracked)
│   ├── base_modified.txt  18 office-agent–modified base files
│   └── base_added.txt     65 office-agent–added files
├── build/installer/       Windows installer source (build.ps1, OfficeAgent.iss, …)
├── PLAN.md                architecture, MCP inventory, risks, phased plan
└── .docs/                 per-phase build manual
```

## Slim repo

`agent-core/` is a vendored copy of the Hermes fork (`LynxLabVN/agent-core @88d1d62`).
Only the **office-agent delta** is tracked in git (83 files: 18 modified + 65
added). The 5927 byte-identical vanilla-Hermes files are **not tracked** — they
are restored into the working tree (gitignored) by:

```bash
python tools/provision_base.py dev            # download + restore base into agent-core/
# or:  python tools/provision_base.py dev --base-dir /path/to/extracted-fork
cd agent-core && uv pip install -e '.[mcp,acp,dev]'   # then set up the venv
```

Run this once after a fresh clone (or whenever the base files are missing).

## Build

### 1. Build the components (build machine: needs `cargo`, `npm`, `uv`, network)

```bash
# Rust MCP servers (10 binaries)
cd optional-mcps && cargo build --workspace --release      # → target/release/*.exe
# Rust tests
cargo test --workspace --release

# Python mcp-cv-screen (light deps, no torch)
cd ../optional-mcps/mcp-cv-screen
uv venv --python 3.13 .venv
uv pip install --python .venv/Scripts/python.exe fastmcp pymupdf pillow httpx anyio pytest
uv pip install --python .venv/Scripts/python.exe -e . --no-deps

# Web dashboard
cd ../../agent-core/web && npm install && npm run build     # → ../hermes_cli/web_dist/
```

### 2. Build the Windows installer (needs `uv`, `winget`, the parts above)

```powershell
winget install JRSoftware.InnoSetup                 # one time
python tools/provision_base.py dev                  # ensure base is present
powershell -ExecutionPolicy Bypass -File build\installer\build.ps1
```

→ **`build/installer/dist/OfficeAgent-Setup.exe`** — a single offline installer
(~201 MB) that bundles portable Python 3.13, all runtime wheels, the 10 Rust
MCP `.exe`s, ffmpeg, the built dashboard, and the office-agent delta. End users
run it with no network and no manual setup: it creates the venv, offline-pip-
installs everything, overlays the delta, copies skills, writes `config.yaml` with
all 11 MCPs pre-registered (`enabled: true`), and adds a shortcut that runs
`eir dashboard --skip-build`.

Installer details: [`build/installer/README.md`](build/installer/README.md).

## Use

After install (or in the dev venv):

```bash
eir dashboard                       # open the dashboard (http://127.0.0.1:9119/)
eir mcp list                        # list configured MCP servers
eir mcp test <name>                 # test a connection
eir mcp call <server> <tool> [json] # call any MCP tool from the CLI, e.g.
eir mcp call mcp-catalog list_products
eir mcp call mcp-catalog search_catalog "{\"query\":\"LED\"}"
eir mcp call mcp-hr-data create_job '{"title":"Audio Engineer","dept":"Production","jd_markdown":"...","skills_required":["audio"],"exp_level":"mid","location":"Hanoi"}'
```

### MCPs

| MCP | Lang | Auth | Notes |
|-----|------|------|-------|
| mcp-catalog, mcp-ledger, mcp-hr-data | Rust | none | SQLite; catalog auto-seeds products |
| mcp-video-edit | Rust | none | uses bundled ffmpeg (`EDIT_FFMPEG` env) |
| mcp-trend-research | Rust | none | trending hooks/audio |
| mcp-cv-screen | Python | none | CV parse/score/summarize; video-portfolio stubbed (no torch) |
| mcp-schedule | Rust | Cal.com API key | OAuth-gated |
| mcp-social-{youtube,meta,tiktok}, mcp-zalo-oa | Rust | OAuth2 | enabled with `PLACEHOLDER` tokens — fill real secrets in `%LOCALAPPDATA%\hermes\config.yaml` |

The 6 no-auth MCPs work out of the box; the 5 OAuth-gated ones start and expose
their tools but need real credentials for live API calls.

## Conventions

- No modification to `agent-core/run_agent.py` or `toolsets.py` (sacred core).
- Third-party integrations ship as standalone MCP crates under `optional-mcps/`.
- Adding a new office-agent modification to a base file: edit it, then move its
  path from `tools/base_files.txt` → `tools/base_modified.txt`, regenerate the
  `# BEGIN hermes-base-files` block in `agent-core/.gitignore`, and `git add`
  the file. (Post-slim, the classifier only sees tracked files, so do this
  manually rather than re-running `classify_delta.py` for the base list.)

## More

- [`PLAN.md`](PLAN.md) — architecture, MCP inventory, risks, integration rules.
- [`.docs/`](.docs/) — per-phase build manual (Phase 0–6).
- [`build/installer/README.md`](build/installer/README.md) — installer build & post-install details.