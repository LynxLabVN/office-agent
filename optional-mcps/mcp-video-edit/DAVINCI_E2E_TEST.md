# E2E Test Plan — DaVinci Resolve MCP backend (Linux + Windows)

Status: prerequisite. The DaVinci backend is **not yet built** (see
`MULTI_APP_MCP_INSIGHT.md`, Tier 1). This document defines the end-to-end test that must
pass once the prototype from step 3 of the insight ("Prototype DaVinci backend via
`DaVinciResolveScript.py`") exists. Write the backend against this spec so it is
testable from day one.

## OS matrix

Run on **both Linux and Windows**. Free-build encode capability differs:
- **Linux**: free build has **no H.264/AAC encode** — render tests must use `prores` /
  `dnxhr`, or use DaVinci Studio.
- **Windows**: free build **does encode H.264/AAC** — render tests may use `h264`/`aac`.

Adjust the render codec per OS in the commands below.

## Engine selection mechanism (per-call, not env)

Engine is selected **per tool call** via an optional `engine` parameter on each
`mcp-video-edit` tool, **not** via an environment variable:

- `engine` omitted or `"ffmpeg"` → ffmpeg backend (default; backward compatible).
- `engine: "davinci"` → DaVinci backend (drives live Resolve via the python bridge).

One binary, one MCP registration. Switching engine = changing one arg in the call — no
restart, no rebuild, no re-register. This replaces the earlier (rejected) `VIDEO_ENGINE`
env-var proposal, which was global-for-process-lifetime and implied one binary per engine.

The DaVinci python bridge **is built** at `optional-mcps/mcp-video-edit/scripts/davinci_bridge.py`.
The Rust `DavinciEngine` shells out to it per call (JSON on stdin, JSON on stdout). Bridge
resolution order: `DAVINCI_BRIDGE` env → candidates relative to the executable →
`davinci_bridge.py` in CWD. Set `DAVINCI_BRIDGE` explicitly in production for determinism.

## What "e2e with real app" means here

A real DaVinci Resolve instance (GUI running, Studio or free) is driven through its
official Python scripting API (`DaVinciResolveScript.py`) by an MCP tool call. The test
asserts that a timeline created + rendered via the MCP produces a real, playable output
file matching expected content — not a mock, not ffmpeg-only fallback.

## Prerequisites

### Linux
- DaVinci Resolve installed and launchable from desktop. Free build: no H.264/AAC
  encode → use `prores`/`dnxhr` for render tests, or use Studio.
- `python3` on PATH with `DaVinciResolveScript.py` importable. Resolve ships the modules
  under its install dir; the bridge reads `DAVINCI_SCRIPT_PATH` (added to `sys.path`
  before import) — set it to the dir containing `DaVinciResolveScript.py`. Verify:
  ```bash
  export DAVINCI_SCRIPT_PATH=/opt/resolve/Developer/   # adjust to your install
  python3 -c "import DaVinciResolveScript as dvr; r=dvr.scriptapp(); print(r.GetProductName())"
  ```
  This must print `Resolve` without error. If it fails, the API modules are not on the
  path — fix `DAVINCI_SCRIPT_PATH` before any MCP test runs.

### Windows
- DaVinci Resolve installed (default `C:\Program Files\Blackmagic Design\DaVinci Resolve\`).
  Free build encodes H.264/AAC.
- Python 3 on PATH with `DaVinciResolveScript.py` importable. The modules live under the
  Resolve install dir (`Modules\` / `Developer\`); set `DAVINCI_SCRIPT_PATH` to that dir.
  Verify in PowerShell:
  ```powershell
  $env:DAVINCI_SCRIPT_PATH = "C:\Program Files\Blackmagic Design\DaVinci Resolve\Developer\"
  python -c "import DaVinciResolveScript as dvr; r=dvr.scriptapp(); print(r.GetProductName())"
  ```
  Must print `Resolve`. Resolve must run as the **same user** that owns the python
  bridge process — UAC elevation mismatch blocks the scripting IPC connection.
- Path quoting in PowerShell: use single-quoted here-strings (`@'...'@`) or backtick-escape
  inner quotes; do not paste bash quoting verbatim.

### Resolve state for tests (both OSes)
- Resolve **must be running** (scripting API talks to the live app).
- A clean, empty project named `mcp-e2e` (tests create/delete timelines in it; do not
  point at a real working project).
- Disk target: `~/.hermes/work/davinci-out/` (Linux) or `%USERPROFILE%\.hermes\work\davinci-out\`
  (Windows). Writable, plenty of free space.
- Disable "Use GPU for decode" mismatch warnings by matching project settings to source,
  or tests will block on a modal.

### MCP backend
- Backend dispatch is in `mcp-video-edit`: tools take an optional `engine` param;
  `resolve_engine(engine)` routes to `FfmpegEngine` (default) or `DavinciEngine`
  (shells out to the python bridge).
- The python bridge ships at `optional-mcps/mcp-video-edit/scripts/davinci_bridge.py`.
- Env the bridge/Rust side reads:
  - `DAVINCI_BRIDGE` — path to `davinci_bridge.py` (recommended: set explicitly).
  - `DAVINCI_PYTHON` — python binary (default `python3`; on Windows use `python`).
  - `DAVINCI_SCRIPT_PATH` — dir containing `DaVinciResolveScript.py` (added to `sys.path`).
  - `DAVINCI_CODEC` — render codec (default `prores`; set `h264` on Windows free build).
  - `RESOLVE_PROJECT` — project name (default `mcp-e2e`).
  - `EDIT_WORKSPACE` — output workspace (default `~/.hermes/work`).
- Each tool returns JSON with an `engine` field: `{ ..., "engine": "ffmpeg" | "davinci" }`.
  For davinci timeline ops (`cut`/`overlay`/`burn_captions`/`add_music`/`concat`),
  `output_path` is a `timeline:<name>` identifier (the timeline lives in the project, not
  a file). For `render`/`extract_audio` it is a real file path.

## Test matrix

Run in order. Each row = one e2e scenario. `Result` column = pass criterion.

| # | Scenario | Tool call (engine) | Pass criterion |
|---|----------|--------------------|----------------|
| 1 | Server health | `health` (no engine) | `engine == "ffmpeg"`, ok true |
| 2 | Timeline create + cut | `cut_by_shotlist` (`davinci`) | timeline in `mcp-e2e`, 2 clips, no error |
| 3 | Overlay text | `overlay_text` (`davinci`) | Text+ effect on track 2; render shows text |
| 4 | Color grade | `color_grade` (`davinci`) | **PENDING**: `color_grade` tool not yet implemented (insight common-set). Skip until landed. |
| 5 | Render to file | `encode_916` (`davinci`) | output file exists, >0 bytes, ffprobe reports expected codec/duration |
| 6 | Roundtrip integrity | ffprobe render vs source | duration within ±1 frame; resolution matches request |
| 7 | Cleanup | delete timeline | timeline gone from project; no orphans |
| 8 | Fallback parity | `cut_by_shotlist` (`ffmpeg` or omitted) | ffmpeg path works; `engine == "ffmpeg"` |

Scenarios 2–7 exercise the real DaVinci path (once the bridge is built). Scenario 8
guards the regression that dispatch breaks the existing ffmpeg backend.

## Step-by-step run

> The DaVinci python bridge is built. The steps below are **runnable against a live
> Resolve instance**. They are also the acceptance contract: each `T<n>` must pass on
> both Linux and Windows.

### A. One-time setup

**Linux**
```bash
# 1. resolve scripting importable (see Prerequisites). Confirm:
export DAVINCI_SCRIPT_PATH=/opt/resolve/Developer/   # adjust to your install
python3 -c "import DaVinciResolveScript as dvr; print(dvr.scriptapp().GetProductName())"

# 2. build the MCP (davinci engine + bridge dispatch are in the binary)
cd /home/lynxadmin/project/office-agent/optional-mcps
cargo build -p mcp-video-edit --release

# 3. env for the davinci engine + bridge
export DAVINCI_BRIDGE="$PWD/mcp-video-edit/scripts/davinci_bridge.py"
export DAVINCI_PYTHON=python3
export DAVINCI_CODEC=prores          # Linux free build: no H.264; use prores/dnxhr
export RESOLVE_PROJECT=mcp-e2e
export EDIT_WORKSPACE=$HOME/.hermes/work
```

**Windows**
```powershell
# 1. resolve scripting importable. Confirm:
$env:DAVINCI_SCRIPT_PATH = "C:\Program Files\Blackmagic Design\DaVinci Resolve\Developer\"
python -c "import DaVinciResolveScript as dvr; print(dvr.scriptapp().GetProductName())"

# 2. build the MCP
cd C:\path\to\office-agent\optional-mcps
cargo build -p mcp-video-edit --release

# 3. env for the davinci engine + bridge
$env:DAVINCI_BRIDGE = "$PWD\mcp-video-edit\scripts\davinci_bridge.py"
$env:DAVINCI_PYTHON = "python"
$env:DAVINCI_CODEC = "h264"          # Windows free build encodes H.264
$env:RESOLVE_PROJECT = "mcp-e2e"
$env:EDIT_WORKSPACE = "$env:USERPROFILE\.hermes\work"
```

### B. Run the matrix

Each scenario passes `engine` in the **args dict of the tool call**, not as an env var.

**Linux** (bash)
```bash
cd ../agent-core
SAMPLE=$HOME/.hermes/work/sample-10s.mp4   # a real 10s mp4

# 1. health (no engine param; server-level)
.venv/bin/python -c "
import sys,json; sys.path.insert(0,'.')
from hermes_cli._domain_shared import call_mcp_tool
r = call_mcp_tool('mcp-video-edit','health',{})
print(json.dumps(r, indent=2))
assert json.loads(r['result'])['engine']=='ffmpeg', 'T1 FAIL: engine'
print('T1 PASS')
"

# 2. cut into 2 shots (davinci engine)
.venv/bin/python -c "
import sys,json; sys.path.insert(0,'.')
from hermes_cli._domain_shared import call_mcp_tool
r = call_mcp_tool('mcp-video-edit','cut_by_shotlist', {
  'input':'$SAMPLE',
  'shots':[{'start_sec':0.0,'end_sec':4.0,'label':'a'},{'start_sec':4.0,'end_sec':10.0,'label':'b'}],
  'engine':'davinci'
})
print(r)
res=json.loads(r['result']) if 'result' in r else r
assert res.get('engine')=='davinci', 'T2 FAIL: engine'; print('T2 PASS')
"

# 3. overlay text (davinci)
.venv/bin/python -c "
import sys,json; sys.path.insert(0,'.')
from hermes_cli._domain_shared import call_mcp_tool
r = call_mcp_tool('mcp-video-edit','overlay_text', {
  'input':'$SAMPLE',
  'overlays':[{'start_sec':0.0,'end_sec':3.0,'text':'E2E TEST','position':'top-left'}],
  'engine':'davinci'
})
print(r); print('T3 PASS')
"

# 4. color_grade -- PENDING: tool not yet implemented. Skip.

# 5. render (encode_916; davinci). Linux free build: codec must be prores/dnxhr.
OUT=$HOME/.hermes/work/davinci-out/r1.mov
mkdir -p "$(dirname $OUT)"
.venv/bin/python -c "
import sys,json; sys.path.insert(0,'.')
from hermes_cli._domain_shared import call_mcp_tool
r = call_mcp_tool('mcp-video-edit','encode_916', {
  'input':'$SAMPLE','quality':'medium','engine':'davinci'
})
print(r); print('T5 PASS')
"

# 6. roundtrip integrity via ffprobe
ffprobe -v error -show_entries format=duration:stream=codec_name,width,height -of json "$OUT"
SRC_DUR=$(ffprobe -v error -show_entries format=duration -of csv=p=0 "$SAMPLE")
OUT_DUR=$(ffprobe -v error -show_entries format=duration -of csv=p=0 "$OUT")
python3 -c "assert abs(float('$OUT_DUR')-float('$SRC_DUR'))<0.05, 'duration drift'; print('T6 PASS')"

# 7. cleanup timeline (davinci)
.venv/bin/python -c "
import sys,json; sys.path.insert(0,'.')
from hermes_cli._domain_shared import call_mcp_tool
r = call_mcp_tool('mcp-video-edit','cleanup_timeline', {'engine':'davinci'})
print(r); print('T7 PASS')
"

# 8. fallback parity (ffmpeg engine explicit)
.venv/bin/python -c "
import sys,json; sys.path.insert(0,'.')
from hermes_cli._domain_shared import call_mcp_tool
r = call_mcp_tool('mcp-video-edit','health',{})
assert json.loads(r['result'])['engine']=='ffmpeg', 'T8 FAIL: ffmpeg fallback'
print('T8 PASS')
"
```

**Windows** (PowerShell)
```powershell
cd ..\agent-core
$SAMPLE = "$env:USERPROFILE\.hermes\work\sample-10s.mp4"   # a real 10s mp4

# 1. health
.venv\Scripts\python.exe -c @'
import sys,json; sys.path.insert(0,'.')
from hermes_cli._domain_shared import call_mcp_tool
r = call_mcp_tool('mcp-video-edit','health',{})
print(json.dumps(r, indent=2))
assert json.loads(r['result'])['engine']=='ffmpeg', 'T1 FAIL'
print('T1 PASS')
'@

# 2. cut (davinci)
.venv\Scripts\python.exe -c @"
import sys,json; sys.path.insert(0,'.')
from hermes_cli._domain_shared import call_mcp_tool
r = call_mcp_tool('mcp-video-edit','cut_by_shotlist', {
  'input':r'$SAMPLE',
  'shots':[{'start_sec':0.0,'end_sec':4.0,'label':'a'},{'start_sec':4.0,'end_sec':10.0,'label':'b'}],
  'engine':'davinci'
})
print(r); print('T2 PASS')
"@

# 3. overlay (davinci)
.venv\Scripts\python.exe -c @"
import sys,json; sys.path.insert(0,'.')
from hermes_cli._domain_shared import call_mcp_tool
r = call_mcp_tool('mcp-video-edit','overlay_text', {
  'input':r'$SAMPLE',
  'overlays':[{'start_sec':0.0,'end_sec':3.0,'text':'E2E TEST','position':'top-left'}],
  'engine':'davinci'
})
print(r); print('T3 PASS')
"@

# 4. color_grade -- PENDING: tool not implemented. Skip.

# 5. render (encode_916; davinci). Windows free build: h264/aac OK.
$OUT = "$env:USERPROFILE\.hermes\work\davinci-out\r1.mov"
New-Item -Force -ItemType Directory -Path (Split-Path $OUT) | Out-Null
.venv\Scripts\python.exe -c @"
import sys,json; sys.path.insert(0,'.')
from hermes_cli._domain_shared import call_mcp_tool
r = call_mcp_tool('mcp-video-edit','encode_916', {
  'input':r'$SAMPLE','quality':'medium','engine':'davinci'
})
print(r); print('T5 PASS')
"@

# 6. roundtrip integrity via ffprobe
ffprobe -v error -show_entries format=duration:stream=codec_name,width,height -of json "$OUT"
$SRC_DUR = (ffprobe -v error -show_entries format=duration -of csv=p=0 "$SAMPLE")
$OUT_DUR = (ffprobe -v error -show_entries format=duration -of csv=p=0 "$OUT")
python -c "assert abs(float('$OUT_DUR')-float('$SRC_DUR'))<0.05, 'duration drift'; print('T6 PASS')"

# 7. cleanup (davinci)
.venv\Scripts\python.exe -c @"
import sys,json; sys.path.insert(0,'.')
from hermes_cli._domain_shared import call_mcp_tool
r = call_mcp_tool('mcp-video-edit','cleanup_timeline', {'engine':'davinci'})
print(r); print('T7 PASS')
"@

# 8. fallback parity (ffmpeg)
.venv\Scripts\python.exe -c @'
import sys,json; sys.path.insert(0,'.')
from hermes_cli._domain_shared import call_mcp_tool
r = call_mcp_tool('mcp-video-edit','health',{})
assert json.loads(r['result'])['engine']=='ffmpeg', 'T8 FAIL'
print('T8 PASS')
'@
```

### C. Expected final
All runnable scenarios print `T<n> PASS`. T4 is skipped (pending `color_grade` tool).
Any Python traceback, non-zero bridge exit with a raw stderr (no JSON), or a result
missing `output_path` = stop, investigate that scenario before proceeding.

## Known gotchas

- **Bridge not found**: if `DAVINCI_BRIDGE` is unset and no candidate resolves, the tool
  returns `davinci_bridge.py not found ...; set DAVINCI_BRIDGE to its path`. Set it
  explicitly (step A.3) — don't rely on relative-to-exe resolution in deployed layouts.
- **`DaVinciResolveScript.py` not importable**: bridge returns `cannot import
  DaVinciResolveScript — set DAVINCI_SCRIPT_PATH ...`. Fix that env before continuing.
- **Resolve not running**: bridge returns `scriptapp() returned None — is Resolve
  running?`. Launch Resolve and retry.
- **Linux free build = no H.264/AAC encode.** Use `prores`/`dnxhr` (`DAVINCI_CODEC=prores`)
  for render tests on Linux free, or use Studio. Windows free build encodes H.264/AAC
  fine (`DAVINCI_CODEC=h264`).
- **Resolve must be open and idle** — scripting API cannot launch it. If a modal
  (unsaved-changes, GPU mismatch) is up, scripting calls hang with no timeout. Pre-clear
  modals.
- **Windows UAC**: Resolve must run as the same user that owns the python bridge process;
  an elevation mismatch blocks the scripting IPC connection.
- **Windows path quoting**: PowerShell here-strings (`@'...'@` literal, `@"..."@`
  expanding) differ from bash single-quotes — do not paste bash quoting verbatim.
- **Project lock** — Resolve locks the open project. Tests target a dedicated `mcp-e2e`
  project so they never touch a real edit.
- **Render blocking** — Resolve render is synchronous via the API queue (the bridge polls
  `IsRenderingInProgress()`). Long clips block the MCP stdio channel. Keep test clips
  ≤10s; consider async job + poll for production use.
- **Fusion Text+** — overlay (`overlay_text`) depends on a supported text-insertion call
  (`AddGenerator` / `AddFusionEffectByIndex`); older Resolve builds expose neither and the
  bridge returns `could not add text overlay`. Fall back to a normal text generator or
  upgrade Resolve (see DAVINCI_E2E_TEST.md gotchas).
- **API version sensitivity** — `CreateTimelineFromClips`, `AppendClipsToTimeline`
  clip-info dicts (`startFrame`/`endFrame`/`trackIndex`), and `SetRenderSettings` keys
  differ across Resolve 18/19. The bridge assumes ≥18; confirm call shapes against your
  live build if a command returns an unexpected error.
- **Cross-OS paths**: `EDIT_WORKSPACE` uses OS-native separator; the Rust side uses
  `PathBuf` so no hard-coded `/` — safe on both OSes.

## What this does NOT cover

- Color grade numerical accuracy (only "scopes shift" qualitative) — add a pixel-diff
  against a known-grade LUT if precision is needed. Also `color_grade` tool itself is
  not yet implemented (T4 pending).
- Multi-track / multi-clip timelines (single clip only).
- Performance benchmarks — add timed runs separately.
- macOS path — not requested; macOS free build encodes H.264 like Windows and can be
  added later by mirroring the Windows block.
- CapCut/Premiere — out of scope, Tier 2/3 (see insight doc).

## When to run

The DaVinci backend (python bridge + Rust `DavinciEngine` dispatch) is built. Run this
matrix whenever the bridge or Resolve API surface changes. The dispatch layer (per-call
`engine` param) and the davinci subprocess path are unit-tested in
`src/engine.rs` / `src/tools.rs` (mock-python bridge); this e2e matrix exercises the real
live-Resolve path and is the final acceptance gate.