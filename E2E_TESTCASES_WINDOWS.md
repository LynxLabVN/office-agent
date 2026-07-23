# E2E Test Cases — Windows (Manual)

Companion to [E2E_TEST_WINDOWS.md](E2E_TEST_WINDOWS.md).
Each test case: ID, title, priority, preconditions, steps, expected result, status.
Status: `[ ]` untested, `[x]` pass, `[~]` fail, `[!]` blocked.

Priority: **P0** blocker / must pass, **P1** important, **P2** nice-to-have.

> **Context:** Dev runs in WSL. Test target is Windows native. Section 1 builds the `.exe`/bundle artifacts in WSL and copies them to `C:\office-agent-test\`. Sections 2–9 then run **only** on Windows against the copied artifacts.

---

## 1. Build & delivery (WSL → Windows)

### TC-BLD-01 — Rust MCPs cross-build
- **Priority:** P0
- **Preconditions:** WSL, `rustup target x86_64-pc-windows-msvc` + linker.
- **Steps:**
  1. `cd optional-mcps`
  2. `cargo build --release --target x86_64-pc-windows-msvc`
- **Expected:** `mcp-catalog.exe`, `mcp-ledger.exe`, `mcp-video-edit.exe` produced under `target/x86_64-pc-windows-msvc/release/`.
- **Status:** [ ]

### TC-BLD-02 — Hermes CLI PyInstaller bundle
- **Priority:** P0
- **Preconditions:** PyInstaller in venv, web_dist already built.
- **Steps:**
  1. `cd agent-core`
  2. `pyinstaller --name hermes --collect-all hermes_agent --add-data "hermes_cli/web_dist:hermes_cli/web_dist" run_agent.py`
- **Expected:** `dist/hermes/` (or `dist/hermes.exe`) produced; no missing-import errors in build log.
- **Status:** [ ]

### TC-BLD-03 — Web dashboard build
- **Priority:** P0
- **Preconditions:** Node installed in WSL.
- **Steps:**
  1. `cd agent-core/web && npm install && npm run build`
- **Expected:** `hermes_cli/web_dist/index.html` exists.
- **Status:** [ ]

### TC-BLD-04 — Copy artifacts to Windows staging
- **Priority:** P0
- **Steps:**
  1. `cp` Hermes bundle, MCP exes, web_dist, node_modules to `/mnt/c/office-agent-test/`.
- **Expected:** All files present under `C:\office-agent-test\`; no permission errors.
- **Status:** [ ]

### TC-BLD-05 — No WSL paths baked into bundle
- **Priority:** P0
- **Steps:**
  1. On Windows: `Select-String -Path C:\office-agent-test\hermes\*.exe,C:\office-agent-test\mcps\*.exe -Pattern "/home/"`.
- **Expected:** No matches, or only benign references. No `/home/lynxadmin/...` absolute paths.
- **Status:** [ ]

### TC-BLD-06 — Windows host prereqs installed
- **Priority:** P0
- **Steps:**
  1. Verify `ffmpeg.exe`, Node 20+, `~\.hermes\.env` with LLM key on Windows.
  2. Register MCPs via `hermes mcp add` with `C:\office-agent-test\mcps\...exe` paths.
- **Expected:** All present; `hermes mcp list` shows MCPs registered with `C:\` paths, not WSL paths.
- **Status:** [ ]

---

## 2. Pre-flight smoke

### TC-PRE-01 — Hermes CLI version
- **Priority:** P0
- **Preconditions:** Section 1 artifacts copied to `C:\office-agent-test\`.
- **Steps:**
  1. `cd C:\office-agent-test\hermes`
  2. `.\hermes.exe --version`
- **Expected:** Version string printed, no Python traceback.
- **Status:** [ ]

### TC-PRE-02 — MCP servers registered with Windows paths
- **Priority:** P0
- **Preconditions:** MCPs registered via `mcp add` with `C:\office-agent-test\mcps\...exe`.
- **Steps:**
  1. `.\hermes.exe mcp list`
- **Expected:** `mcp-catalog`, `mcp-ledger`, `mcp-video-edit`, `mcp-cv-screen` listed and not failed; `command` paths start with `C:\`, not `/home/` or `optional-mcps/`.
- **Status:** [ ]

### TC-PRE-03 — Rust MCP binaries start
- **Priority:** P0
- **Preconditions:** Binaries at `C:\office-agent-test\mcps\`.
- **Steps:**
  1. Run `C:\office-agent-test\mcps\mcp-catalog.exe`; observe log; `Ctrl+C`.
  2. Run `C:\office-agent-test\mcps\mcp-ledger.exe`; observe log; `Ctrl+C`.
  3. Run `C:\office-agent-test\mcps\mcp-video-edit.exe`; observe log; `Ctrl+C`.
- **Expected:** Each prints JSON-RPC init log, exits cleanly on `Ctrl+C`.
- **Status:** [ ]

### TC-PRE-04 — Web dashboard bundle present
- **Priority:** P1
- **Preconditions:** Section 1 web_dist copied.
- **Steps:**
  1. `Test-Path C:\office-agent-test\web_dist\index.html`
- **Expected:** Returns `True`.
- **Status:** [ ]

---

## 3. TUI / CLI chat

### TC-TUI-01 — TUI loads
- **Priority:** P0
- **Preconditions:** API key set in `~\.hermes\.env`.
- **Steps:**
  1. `.venv\Scripts\hermes.exe`
- **Expected:** Input box shown; no "Missing API key" banner; status bar shows model.
- **Status:** [ ]

### TC-TUI-02 — Basic chat reply
- **Priority:** P0
- **Preconditions:** TC-TUI-01 passed.
- **Steps:**
  1. Send `Hello, are you there?`
- **Expected:** Agent replies (Vietnamese or English per config); no error toast.
- **Status:** [ ]

### TC-TUI-03 — Slash commands listed
- **Priority:** P1
- **Steps:**
  1. Send `/help`
- **Expected:** Lists slash commands incl. marketing/social skills if present.
- **Status:** [ ]

### TC-TUI-04 — Memory recall
- **Priority:** P1
- **Steps:**
  1. Send `/memory recall last conversation`
- **Expected:** Memory tools load; returns structured result or "no memories yet."
- **Status:** [ ]

### TC-TUI-05 — Marketing script generation
- **Priority:** P0
- **Preconditions:** Marketing skill installed.
- **Steps:**
  1. Send `Create a 30-second TikTok script for product MA5, format UGC, price 199k`
- **Expected:** Output contains 9-section template (Overview, Shoot requirements, Setting, Props, Shot list, Scenes, Text on screen, Shoot notes, Pre-shoot checklist).
- **Status:** [ ]

### TC-TUI-06 — Clean quit
- **Priority:** P1
- **Steps:**
  1. Press `Ctrl+C`, choose "Quit cleanly."
- **Expected:** Exit code 0; no orphaned `python.exe` in Task Manager.
- **Status:** [ ]

---

## 4. Web dashboard

### TC-WEB-01 — Dashboard server starts
- **Priority:** P0
- **Steps:**
  1. `.venv\Scripts\python.exe -m hermes_cli.main web --no-open`
- **Expected:** Listens on `http://127.0.0.1:9119`; no 500 in first 10 log lines.
- **Status:** [ ]

### TC-WEB-02 — Config screen loads
- **Priority:** P0
- **Steps:**
  1. Open `http://127.0.0.1:9119` in browser.
- **Expected:** Login/config screen loads, no blank page.
- **Status:** [ ]

### TC-WEB-03 — StatusPage renders
- **Priority:** P1
- **Steps:**
  1. Navigate to StatusPage.
- **Expected:** Shows agent status and recent sessions.
- **Status:** [ ]

### TC-WEB-04 — ConfigPage save
- **Priority:** P1
- **Steps:**
  1. Open ConfigPage.
  2. Change a non-critical value.
  3. Save.
- **Expected:** Success toast; value persists on reload.
- **Status:** [ ]

### TC-WEB-05 — EnvPage key management
- **Priority:** P1
- **Steps:**
  1. Open EnvPage.
- **Expected:** Shows masked API key names; Save/Clear buttons work.
- **Status:** [ ]

### TC-WEB-06 — No console errors across pages
- **Priority:** P1
- **Steps:**
  1. Open DevTools Console.
  2. Navigate StatusPage → ConfigPage → EnvPage.
- **Expected:** Zero red 500 errors.
- **Status:** [ ]

### TC-WEB-07 — Hard refresh stability
- **Priority:** P2
- **Steps:**
  1. `Ctrl+F5` on each page.
- **Expected:** Each page re-renders correctly.
- **Status:** [ ]

### TC-WEB-08 — Dev HMR (optional)
- **Priority:** P2
- **Preconditions:** Vite dev server running on `:5173`.
- **Steps:**
  1. Open `http://localhost:5173`; confirm StatusPage loads (proxy to `9119`).
  2. Edit text in `web\src\App.tsx`.
- **Expected:** Browser updates within ~2 s via HMR.
- **Status:** [ ]

---

## 5. Gateway / messaging

### TC-GW-01 — Zalo gateway starts
- **Priority:** P0
- **Preconditions:** Zalo plugin installed.
- **Steps:**
  1. `npm run gateway` (or `npm run setup:zalo` first).
- **Expected:** Gateway starts; QR shown for pairing.
- **Status:** [ ]

### TC-GW-02 — Zalo QR pairing
- **Priority:** P0
- **Steps:**
  1. Scan QR with secondary Zalo account.
- **Expected:** Pairing succeeds; gateway log shows connected.
- **Status:** [ ]

### TC-GW-03 — Zalo ping reply
- **Priority:** P0
- **Steps:**
  1. From secondary account send `ping`.
- **Expected:** Bot replies `pong` or alive confirmation.
- **Status:** [ ]

### TC-GW-04 — Zalo script command
- **Priority:** P1
- **Steps:**
  1. Send `Tạo kịch bản MA5 30s UGC`.
- **Expected:** Bot returns short script or clarifying question.
- **Status:** [ ]

### TC-GW-05 — Zalo /help
- **Priority:** P2
- **Steps:**
  1. Send `/help`.
- **Expected:** Bot lists supported commands.
- **Status:** [ ]

### TC-GW-06 — Telegram ping (if configured)
- **Priority:** P1
- **Preconditions:** `TELEGRAM_BOT_TOKEN` set.
- **Steps:**
  1. `.venv\Scripts\hermes.exe gateway`
  2. Send `ping` from Telegram.
- **Expected:** Reply within ~10 s.
- **Status:** [ ]

---

## 6. MCP skills

### TC-MCP-01 — Catalog list
- **Priority:** P0
- **Steps:**
  1. In chat send `List all products in the catalog`.
- **Expected:** `mcp-catalog` invoked; returns MA5, A14, AD35, A8, GX200, P011, bao đàn, UHF, mic đeo tai.
- **Status:** [ ]

### TC-MCP-02 — Ledger record
- **Priority:** P0
- **Steps:**
  1. Send `Record a test post for product MA5 on TikTok with 1,200 views and 45 likes`.
- **Expected:** `mcp-ledger` invoked; returns success/record ID.
- **Status:** [ ]

### TC-MCP-03 — Ledger query back
- **Priority:** P1
- **Steps:**
  1. Send `Get performance for product MA5 this week`.
- **Expected:** Returns the record from TC-MCP-02.
- **Status:** [ ]

### TC-MCP-04 — Video edit create
- **Priority:** P1
- **Preconditions:** `ffmpeg` on `PATH`.
- **Steps:**
  1. Send `Create a 3-second 9:16 test video with text "E2E TEST" in workspace`.
- **Expected:** `mcp-video-edit` invoked; returns file path under `EDIT_WORKSPACE`; file plays in VLC/WMP.
- **Status:** [ ]

---

## 7. Marketing pipeline

### TC-PIPE-01 — Pipeline trigger
- **Priority:** P0
- **Steps:**
  1. Send `/marketing-pipeline new product=MA5 format=UGC duration=30s`.
- **Expected:** Agent confirms product, fetches specs via `mcp-catalog`.
- **Status:** [ ]

### TC-PIPE-02 — FORMAT_PICK
- **Priority:** P1
- **Steps:**
  1. Continue from TC-PIPE-01.
- **Expected:** Agent picks UGC with rationale.
- **Status:** [ ]

### TC-PIPE-03 — SCRIPT_DRAFT
- **Priority:** P0
- **Steps:**
  1. Continue flow.
- **Expected:** Agent outputs 9-section script template.
- **Status:** [ ]

### TC-PIPE-04 — HOOK_ITERATE
- **Priority:** P1
- **Steps:**
  1. Continue flow.
- **Expected:** Agent offers 3 hook variants and asks manager to pick.
- **Status:** [ ]

### TC-PIPE-05 — MANAGER_REVIEW_GATE
- **Priority:** P0
- **Steps:**
  1. Continue flow.
- **Expected:** Agent sends review request via gateway (Zalo/Telegram).
- **Status:** [ ]

### TC-PIPE-06 — SHOOT_BRIEF after approve
- **Priority:** P1
- **Steps:**
  1. Reply mock "approve".
- **Expected:** Agent emits shoot brief.
- **Status:** [ ]

### TC-PIPE-07 — FOOTAGE_INGEST
- **Priority:** P1
- **Preconditions:** Any `.mp4` in `EDIT_WORKSPACE\incoming\`.
- **Steps:**
  1. Tell agent "footage uploaded".
- **Expected:** Agent calls `mcp-video-edit` ingest.
- **Status:** [ ]

### TC-PIPE-08 — AUTO_EDIT
- **Priority:** P1
- **Steps:**
  1. Continue flow.
- **Expected:** Agent calls cut/concat/overlay tools; returns preview path.
- **Status:** [ ]

### TC-PIPE-09 — FINAL_REVIEW_GATE
- **Priority:** P0
- **Steps:**
  1. Continue flow.
- **Expected:** Agent asks for final OK before publish.
- **Status:** [ ]

### TC-PIPE-10 — PUBLISH mock
- **Priority:** P1
- **Steps:**
  1. Mock-approve.
- **Expected:** Agent schedules or claims to post to configured platform.
- **Status:** [ ]

### TC-PIPE-11 — MONITOR ledger
- **Priority:** P1
- **Steps:**
  1. Continue flow.
- **Expected:** Agent records post in `mcp-ledger`.
- **Status:** [ ]

### TC-PIPE-12 — PipelinePage shows piece
- **Priority:** P1
- **Preconditions:** Dashboard server running.
- **Steps:**
  1. Open dashboard `/pipeline`.
- **Expected:** Piece appears in correct Kanban column.
- **Status:** [ ]

---

## 8. Error handling & resilience

### TC-ERR-01 — Missing API key
- **Priority:** P0
- **Steps:**
  1. Rename `~\.hermes\.env` to `.env.bak`.
  2. Start `hermes`, send a prompt.
  3. Restore `.env`.
- **Expected:** Clear "provider API key missing" message; no crash.
- **Status:** [ ]

### TC-ERR-02 — MCP stopped
- **Priority:** P1
- **Steps:**
  1. Kill one MCP binary process.
  2. `hermes mcp list`.
- **Expected:** Stopped MCP shows failed/disconnected; others still listed.
- **Status:** [ ]

### TC-ERR-03 — Bad video file
- **Priority:** P1
- **Steps:**
  1. Place 0-byte `.mp4` in `EDIT_WORKSPACE\incoming\`.
  2. Ask agent to ingest.
- **Expected:** `mcp-video-edit` returns readable error; agent explains to user.
- **Status:** [ ]

### TC-ERR-04 — Gateway offline
- **Priority:** P1
- **Steps:**
  1. Start `hermes` chat with gateway not running.
  2. Ask agent to message manager.
- **Expected:** Agent starts gateway or reports unavailable; no silent failure.
- **Status:** [ ]

---

## 9. Windows-specific

### TC-WIN-01 — Path backslashes in logs
- **Priority:** P2
- **Steps:**
  1. Review logs from prior runs.
- **Expected:** Paths use backslashes or properly escaped.
- **Status:** [ ]

### TC-WIN-02 — No WSL paths
- **Priority:** P2
- **Steps:**
  1. Review logs from native Windows runs.
- **Expected:** No `/home/...` WSL paths.
- **Status:** [ ]

### TC-WIN-03 — Ctrl+C clean termination
- **Priority:** P1
- **Steps:**
  1. `Ctrl+C` in PowerShell on `hermes`, `gateway`, `web`.
- **Expected:** Each terminates cleanly.
- **Status:** [ ]

### TC-WIN-04 — No orphaned processes
- **Priority:** P1
- **Steps:**
  1. After quitting, open Task Manager.
- **Expected:** No leftover `python.exe` / `node.exe` from this app.
- **Status:** [ ]

### TC-WIN-05 — Long filenames
- **Priority:** P2
- **Steps:**
  1. Use a long-named file in `EDIT_WORKSPACE`.
- **Expected:** No Windows path-length error.
- **Status:** [ ]

---

## Summary

| Section | Total | Pass | Fail | Blocked |
|---|---|---|---|---|
| Build & delivery | 6 | | | |
| Pre-flight | 4 | | | |
| TUI | 6 | | | |
| Web | 8 | | | |
| Gateway | 6 | | | |
| MCP | 4 | | | |
| Pipeline | 12 | | | |
| Error handling | 4 | | | |
| Windows | 5 | | | |
| **Total** | **55** | | | |

Failure capture (for each `[~]`):
- Test case ID:
- Command/message triggered:
- Last 30 log lines:
- Screenshot (if UI):