# Office Agent — Windows installer build

Produces a single offline `OfficeAgent-Setup.exe` that bundles portable
Python, all runtime wheels, the 10 Rust MCP `.exe`s, ffmpeg, the built web
dashboard, and the office-agent delta — and auto-connects all no-auth MCPs on
install with zero manual steps.

## Prerequisites (build machine)

- `uv` (Python), `cargo` (Rust), `npm` (Node), `git`, `winget` (to install Inno Setup)
- Already built: `optional-mcps/target/release/*.exe` (10 MCPs) and
  `agent-core/hermes_cli/web_dist/` (`cd agent-core/web && npm install && npm run build`)
- The Hermes base extracted at `%TEMP%\hermes-base\agent-core-88d1d62…`
  (this is where `tools/provision_base.py` / the build pull the base from;
  run the classifier workflow once to materialize it)
- Inno Setup 6: `winget install JRSoftware.InnoSetup`

## Build

```powershell
powershell -ExecutionPolicy Bypass -File build\installer\build.ps1
```

Output: `build\installer\dist\OfficeAgent-Setup.exe` (~250 MB installed).

## What the installer does (post-install, no network)

1. Creates `%LOCALAPPDATA%\hermes` (HERMES_HOME) + data/skills/cache/logs/sessions/work.
2. Builds a venv from the bundled portable Python 3.13.
3. Offline `pip install` the base wheel `[mcp]` + `mcp-cv-screen` (light deps,
   no torch — video-portfolio analysis is stubbed in this build) + the `eir`
   CLI shim wheel.
4. Overlays the office-agent delta (83 files) onto site-packages.
5. Copies the 5 office-agent skill dirs into `HERMES_HOME\skills`.
6. Writes `HERMES_HOME\config.yaml` with all 11 MCPs pre-registered (`enabled: true`).
7. Sets `HERMES_HOME` and appends the bundled ffmpeg dir to the user `PATH`.
8. Start Menu / desktop shortcut runs `eir dashboard --skip-build`
   → opens `http://127.0.0.1:9119/`.

## After install

- The user-facing command is **`eir`** (a shim over the Hermes CLI): `eir
  dashboard`, `eir mcp list`, `eir mcp test <name>`, etc.
- **All MCPs are callable from the CLI** as `eir mcp call <server> <tool>
  [json-args]`, e.g. `eir mcp call mcp-catalog list_products` or
  `eir mcp call mcp-catalog search_catalog "{\"query\":\"LED\"}"`.
- 6 no-auth MCPs work immediately (catalog, ledger, video-edit, hr-data,
  trend-research, cv-screen).
- 5 OAuth-gated MCPs (youtube, meta, tiktok, zalo-oa, schedule) start and
  expose their tools, but real API calls need secrets — edit
  `%LOCALAPPDATA%\hermes\config.yaml` and replace the `PLACEHOLDER_*` values.
- Uninstall removes the venv only; user data (sessions, DBs, skills) is kept.