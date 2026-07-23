# Manual E2E Test Runbook — Windows

Manual end-to-end test instructions for the **office-agent** app on Windows.
Execute steps by hand and tick them off. Do **not** run automated test suites.

> **Cross-environment note:** Development happens inside **WSL** (this repo lives at `/home/lynxadmin/project/office-agent`). The test target is **Windows native** — the built artifacts (`.exe` files + web bundle) must be copied from WSL to the Windows host and run there. See [Section 0](#0-build--delivery-wsl--windows) for the build/copy steps that produce the testable artifacts.

---

## 0. Build & delivery (WSL → Windows)

Goal: build all testable artifacts once in WSL, copy them to a Windows folder, then run the test (Sections 2–9) entirely on Windows native. The "app" has **no single exe** — it is three pieces:

| Piece | Technology | Build command (WSL) | Windows output |
|---|---|---|---|
| Hermes CLI + Python deps | Python 3.13 (PyInstaller) | `pyinstaller` spec | `hermes.exe` (+ bundled runtime) |
| MCP servers | Rust | `cargo build --release --target x86_64-pc-windows-msvc` | `mcp-catalog.exe`, `mcp-ledger.exe`, `mcp-video-edit.exe` |
| Web dashboard | React/Vite | `npm run build` | `web_dist/` (static files) |

### 0.1 Prereqs for cross-build in WSL

```bash
# Rust Windows target + linker
rustup target add x86_64-pc-windows-msvc          # or x86_64-pc-windows-gnu if using MinGW
# PyInstaller in the venv
cd agent-core
.venv/bin/pip install pyinstaller
# Node (already present for web build)
```

> Cross-compiling Rust to `x86_64-pc-windows-msvc` from WSL needs the MSVC linker (`lld-link` or a Windows SDK). Simpler reliable path: **build Rust MCPs on the Windows host** (install Rust there, `cargo build --release`). Use WSL cross-build only if you have the linker set up. Pick one route and record which.

### 0.2 Build Hermes CLI exe (PyInstaller)

In WSL (or on Windows host — pick one):

```bash
cd agent-core
# One-dir bundle (preferred — easier to inspect)
.venv/bin/pyinstaller --noconfirm --name hermes \
  --collect-all hermes_agent \
  --collect-submodules hermes_cli \
  --add-data "hermes_cli/web_dist:hermes_cli/web_dist" \
  run_agent.py
```

Output: `agent-core/dist/hermes/` (folder) or `dist/hermes.exe` (one-file).
If `run_agent.py` is not the entry, use the actual CLI entry (check `pyproject.toml` `[project.scripts]`).

> If PyInstaller misses a hidden import, add `--hidden-import <module>` per failure. Bundled `ffmpeg`/`uv` are **not** included — install them on the Windows host separately.

### 0.3 Build Rust MCP exes

```bash
cd optional-mcps
cargo build --release --target x86_64-pc-windows-msvc
# outputs: target/x86_64-pc-windows-msvc/release/mcp-catalog.exe etc.
```

Or on Windows host:
```powershell
cd optional-mcps
cargo build --release
# outputs: target\release\mcp-catalog.exe etc.
```

### 0.4 Build web dashboard

```bash
cd agent-core/web
npm install
npm run build
# outputs to ../hermes_cli/web_dist/
```

### 0.5 Copy artifacts to Windows

From WSL, copy to a Windows staging folder. Replace `<WINUSER>` and the drive letter with your actual host paths.

```bash
# Example staging: C:\office-agent-test\
WIN_DIR="/mnt/c/office-agent-test"
mkdir -p "$WIN_DIR"

# Hermes bundle
cp -r agent-core/dist/hermes "$WIN_DIR/hermes"

# Rust MCPs
mkdir -p "$WIN_DIR/mcps"
cp optional-mcps/target/x86_64-pc-windows-msvc/release/mcp-catalog.exe   "$WIN_DIR/mcps/"
cp optional-mcps/target/x86_64-pc-windows-msvc/release/mcp-ledger.exe    "$WIN_DIR/mcps/"
cp optional-mcps/target/x86_64-pc-windows-msvc/release/mcp-video-edit.exe "$WIN_DIR/mcps/"

# Web bundle
cp -r agent-core/hermes_cli/web_dist "$WIN_DIR/web_dist"

# Zalo plugin + package.json (for gateway)
cp package.json package-lock.json "$WIN_DIR/"
cp -r node_modules "$WIN_DIR/"  # or run npm install on Windows
```

### 0.6 Windows host prereqs (install once, not part of test)

- Python 3.13 (only if running Hermes from source instead of the PyInstaller bundle).
- `ffmpeg.exe` on `PATH` (for `mcp-video-edit`).
- Node.js 20+ (for Zalo gateway: `npm run gateway`).
- Configure `~\.hermes\.env` (Windows) with LLM provider key.
- Register MCPs on Windows side against the copied `.exe` paths:
  ```powershell
  cd C:\office-agent-test\hermes
  .\hermes.exe mcp add mcp-catalog --command C:\office-agent-test\mcps\mcp-catalog.exe --env CATALOG_DB="$env:USERPROFILE\.hermes\data\catalog.db"
  .\hermes.exe mcp add mcp-ledger    --command C:\office-agent-test\mcps\mcp-ledger.exe    --env LEDGER_DB="$env:USERPROFILE\.hermes\data\ledger.db"
  .\hermes.exe mcp add mcp-video-edit --command C:\office-agent-test\mcps\mcp-video-edit.exe --env EDIT_WORKSPACE="$env:USERPROFILE\.hermes\work"
  ```

From here on, **all test commands run on the Windows host** against the copied artifacts under `C:\office-agent-test\`.

---

## 1. Environment assumptions

- Windows 10/11, PowerShell or Windows Terminal.
- Test artifacts already built in WSL and copied to `C:\office-agent-test\` (see [Section 0](#0-build--delivery-wsl--windows)).
- Python 3.11–3.13, Node.js 20+, `uv` (or the one bundled by `setup.sh` under `.uv`), and `cargo` available on **Windows** (only if you chose to build on the host instead of WSL).
- `.env` / `~\.hermes\.env` configured with at least one LLM provider key (OpenRouter/OpenAI/Nous Portal) so the agent can chat.
- Rust MCP binaries present at `optional-mcps\target\release\mcp-catalog.exe`, `mcp-ledger.exe`, `mcp-video-edit.exe` (built previously, not now).
- Web dashboard bundle exists at `agent-core\hermes_cli\web_dist\` (built previously).

> Do not run `setup.sh`, `npm install`, `cargo build`, or `npm run build` during this test. If those artifacts are missing, the test cannot start.

---

## 2. Pre-flight smoke checks

Run each check and confirm output looks healthy.

1. **Hermes CLI responds**
   ```powershell
   cd C:\office-agent-test\hermes
   .\hermes.exe --version
   ```
   Expected: prints a version string, no Python traceback.

2. **MCP servers are registered** (must be registered on Windows against copied exe paths — see [Section 0.6](#06-windows-host-prereqs-install-once-not-part-of-test))
   ```powershell
   .\hermes.exe mcp list
   ```
   Expected: `mcp-catalog`, `mcp-ledger`, `mcp-video-edit`, and `mcp-cv-screen` listed and **not** red/failed. If the MCP `command` path still points to a WSL path (`/home/...` or `optional-mcps/target/release/`), re-register using `mcp remove` + `mcp add` with the `C:\office-agent-test\mcps\...exe` path.

3. **Rust MCPs start manually**
   ```powershell
   C:\office-agent-test\mcps\mcp-catalog.exe
   ```
   (Press `Ctrl+C` after seeing the JSON-RPC initialization log; repeat for `mcp-ledger` and `mcp-video-edit`.)

4. **Web dashboard static files are present**
   ```powershell
   Test-Path C:\office-agent-test\web_dist\index.html
   ```
   Expected: `True`.

5. **No leftover WSL paths in copied artifacts**
   ```powershell
   Select-String -Path C:\office-agent-test\hermes\*.exe,C:\office-agent-test\mcps\*.exe -Pattern "/home/" -SimpleMatch
   ```
   Expected: no matches (or only benign ones). Any `/home/lynxadmin/...` path baked into the bundle will break on Windows.

---

## 3. TUI / CLI chat E2E

1. Start the agent in a terminal window:
   ```powershell
   cd C:\office-agent-test\hermes
   .\hermes.exe
   ```
2. Wait for the TUI to load. Confirm:
   - Prompt input box appears.
   - No "Missing API key" banner.
   - Status bar shows the configured model.

3. Send the following prompts one by one and observe behavior:

   | # | Prompt | What to verify |
   |---|---|---|
   | 1 | `Hello, are you there?` | Agent replies in Vietnamese or English per config. No error toast. |
   | 2 | `/help` | Lists available slash commands, including any marketing/social skills. |
   | 3 | `/memory recall last conversation` | Memory tools load and return structured results or "no memories yet." |
   | 4 | `Create a 30-second TikTok script for product MA5, format UGC, price 199k` | Agent calls the marketing skill if present; output contains the 9-section template fields (Overview, Shoot requirements, Setting, Props, Shot list, Scenes, Text on screen, Shoot notes, Pre-shoot checklist). |

4. Interrupt with `Ctrl+C` and choose "Quit cleanly."
   Expected: process exits with code `0`, no hanging Python processes in Task Manager.

---

## 4. Web dashboard E2E

### 4.1 Start the dashboard server

In a new PowerShell window:
```powershell
cd C:\office-agent-test\hermes
.\hermes.exe web --no-open
```
(Or if running from source venv: `.venv\Scripts\python.exe -m hermes_cli.main web --no-open`.)

Confirm:
- Server listens on `http://127.0.0.1:9119`.
- No 500 errors in the first 10 log lines.

### 4.2 Open the built dashboard

In a browser:
```
http://127.0.0.1:9119
```

Tick each check:

- [ ] Login/config screen loads without a blank page.
- [ ] **StatusPage** shows agent status and any recent sessions.
- [ ] **ConfigPage** loads the dynamic schema; changing a non-critical value and saving shows a success toast.
- [ ] **EnvPage** displays the configured API key names (masked) and allows "Save / Clear."
- [ ] Browser DevTools Console has **zero** red 500 errors after navigating all three pages.
- [ ] Hard refresh (`Ctrl+F5`) on each page still renders correctly.

### 4.3 Optional dev HMR mode (if dev server already built)

If a `web/` dev server was started separately on `http://localhost:5173`:

- [ ] Open `http://localhost:5173` and confirm the Vite dev page proxies `/api` to `9119` (e.g., StatusPage loads).
- [ ] Make a trivial text change in `agent-core\web\src\App.tsx` and confirm HMR updates the browser within ~2 seconds.

Do **not** build anything; just verify dev mode if it is already running.

---

## 5. Gateway / messaging E2E

### 5.1 Zalo gateway (secondary account required)

1. In a new PowerShell window:
   ```powershell
   npm run gateway
   ```
   or
   ```powershell
   npm run setup:zalo   # if first-time QR pairing is needed
   ```
2. Scan the QR code with a secondary Zalo account.
3. From that secondary account, send these messages to the bot:

   | Message | Expected bot behavior |
   |---|---|
   | `ping` | Replies `pong` or an alive confirmation. |
   | `Tạo kịch bản MA5 30s UGC` | Bot returns a short script or asks clarifying questions. |
   | `/help` | Bot lists supported commands. |

4. Confirm incoming/outgoing messages appear in the gateway log without errors.

### 5.2 Telegram gateway (if configured)

If `TELEGRAM_BOT_TOKEN` is set in `.env`:

1. Start gateway:
   ```powershell
   .venv\Scripts\hermes.exe gateway
   ```
2. From Telegram, send `ping` to the bot.
3. Confirm reply arrives within ~10 seconds.

---

## 6. MCP skill smoke tests

Use the Hermes CLI chat or a direct MCP invocation. Do not run code changes.

### 6.1 Catalog MCP

In chat, send:
```
List all products in the catalog
```

Verify:
- `mcp-catalog` tool is invoked.
- Returns at least the known products: MA5, A14, AD35, A8, GX200, P011, bao đàn, UHF, mic đeo tai.

### 6.2 Ledger MCP

In chat, send:
```
Record a test post for product MA5 on TikTok with 1,200 views and 45 likes
```

Verify:
- `mcp-ledger` tool is invoked.
- Tool returns a success/record ID.
- Query it back: `Get performance for product MA5 this week`.

### 6.3 Video edit MCP

If `ffmpeg` is on `PATH`:

In chat, send:
```
Create a 3-second 9:16 test video with text "E2E TEST" in workspace
```

Verify:
- `mcp-video-edit` is invoked.
- Tool returns a file path under the configured `EDIT_WORKSPACE`.
- The file exists and is playable in Windows Media Player / VLC.

---

## 7. Marketing pipeline full flow (human gates)

Simulate one complete content piece without producing a real video.

1. In TUI or Zalo, trigger:
   ```
   /marketing-pipeline new product=MA5 format=UGC duration=30s
   ```
2. Walk through the human gates from `PLAN.md`:

   | Gate | Action to verify |
   |---|---|
   | **PRODUCT_SELECT** | Agent confirms product MA5 and fetches specs from `mcp-catalog`. |
   | **FORMAT_PICK** | Agent picks UGC with a brief rationale. |
   | **SCRIPT_DRAFT** | Agent outputs the 9-section script template. |
   | **HOOK_ITERATE** | Agent offers 3 hook variations and asks the manager to pick. |
   | **MANAGER_REVIEW_GATE** | Agent sends a review request via the gateway (Zalo/Telegram message to the manager). |
   | **SHOOT_BRIEF** | After a mock "approve" reply, agent emits a shoot brief. |
   | **FOOTAGE_INGEST** | Place any `.mp4` file in `EDIT_WORKSPACE\incoming\`; tell agent "footage uploaded." Agent should call `mcp-video-edit` `ingest`. |
   | **AUTO_EDIT** | Agent calls cut/concat/overlay tools and returns a preview path. |
   | **FINAL_REVIEW_GATE** | Agent asks for final OK before publish. |
   | **PUBLISH** | Mock-approve; agent schedules or claims to post to the configured social platform. |
   | **MONITOR** | Agent records the post in `mcp-ledger`. |

3. Open the dashboard **PipelinePage** (`/pipeline`) and confirm the piece appears in the correct Kanban column.

---

## 8. Error handling & resilience

Deliberately trigger a few failure modes and confirm graceful degradation.

| Test | Step | Expected behavior |
|---|---|---|
| **No API key** | Temporarily rename `~\.hermes\.env` to `.env.bak`, start `hermes`, send a prompt. | Agent shows a clear "provider API key missing" message, does not crash. Restore the file after. |
| **Missing MCP** | Stop one MCP binary, run `hermes mcp list`. | The stopped MCP shows failed/disconnected; other MCPs still listed. |
| **Bad video file** | Place a 0-byte `.mp4` in `EDIT_WORKSPACE\incoming\` and ask agent to ingest. | `mcp-video-edit` returns a readable error; agent explains it to the user. |
| **Gateway offline** | Start `hermes` chat while gateway is not running, ask it to message a manager. | Agent either starts gateway or reports it is unavailable, instead of silently failing. |

---

## 9. Windows-specific checks

- [ ] File paths in logs use backslashes or are properly escaped.
- [ ] No `WSL` paths appear in Windows-native runs (e.g., `/home/...`).
- [ ] `Ctrl+C` in PowerShell cleanly terminates `hermes` / `gateway` / `web` processes.
- [ ] Check Task Manager: after quitting, no orphaned `python.exe` or `node.exe` processes remain from this app.
- [ ] Long filenames in `EDIT_WORKSPACE` do not trigger Windows path-length errors.

---

## 10. Sign-off

Record results for each section:

| Section | Pass / Fail | Notes |
|---|---|---|
| Pre-flight | | |
| TUI chat | | |
| Web dashboard | | |
| Zalo gateway | | |
| Telegram gateway | | |
| MCP skills | | |
| Marketing pipeline | | |
| Error handling | | |
| Windows-specific | | |

If any section fails, capture:
1. The exact command/message that triggered it.
2. The last 30 lines of the relevant PowerShell log.
3. A screenshot of the dashboard/console if UI-related.

---

### Quick command cheat sheet

```powershell
# All run on Windows host against copied artifacts:
cd C:\office-agent-test\hermes
.\hermes.exe --version
.\hermes.exe mcp list
.\hermes.exe
# Web server (from Python entry inside bundle, or source venv):
.\hermes.exe web --no-open
# Zalo gateway (from staging node_modules):
cd C:\office-agent-test
npm run gateway
```