# ============================================================================
# office-agent Windows installer build orchestrator.
# Run on the BUILD machine (has network + cargo + npm + uv + ISCC).
#   powershell -ExecutionPolicy Bypass -File build\installer\build.ps1
# Produces: build\installer\dist\OfficeAgent-Setup.exe
# ============================================================================
[CmdletBinding()]
param(
    [switch]$SkipArtifactCheck
)

$ErrorActionPreference = "Stop"
$Root      = (Resolve-Path "$PSScriptRoot\..\..").Path
$Inst      = Join-Path $Root "build\installer"
$Payload   = Join-Path $Inst "payload"
$Wheels    = Join-Path $Payload "wheels"
$Dist      = Join-Path $Inst "dist"
$Rel       = Join-Path $Root "optional-mcps\target\release"

function Write-Step($m) { Write-Host "`n==> $m" -ForegroundColor Cyan }
function Ensure-Dir($p) { New-Item -ItemType Directory -Force -Path $p | Out-Null }

# --- 0. clean + dirs ----------------------------------------------------------
Write-Step "Prepare payload dirs"
Remove-Item -Recurse -Force $Payload -ErrorAction SilentlyContinue
Ensure-Dir $Payload
Ensure-Dir $Wheels
Ensure-Dir $Dist

# --- 1. verify artifacts ------------------------------------------------------
if (-not $SkipArtifactCheck) {
    Write-Step "Verify build artifacts"
    $mcps = @("mcp-catalog","mcp-ledger","mcp-video-edit","mcp-hr-data",
              "mcp-schedule","mcp-social-youtube","mcp-social-meta","mcp-social-tiktok",
              "mcp-trend-research","mcp-zalo-oa")
    foreach ($n in $mcps) {
        $exe = Join-Path $Rel "$n.exe"
        if (-not (Test-Path $exe)) { throw "Missing Rust MCP: $exe (run: cargo build --workspace --release)" }
    }
    if (-not (Test-Path "$Root\agent-core\hermes_cli\web_dist\index.html")) {
        throw "Missing web_dist (run: cd agent-core/web && npm install && npm run build)"
    }
    Write-Host "  artifacts OK"
}

# --- 2. build base + cv-screen wheels ----------------------------------------
Write-Step "Build base + cv-screen wheels"
$BaseSrc = Join-Path $env:TEMP "hermes-base\agent-core-88d1d6206f399c134d1f4c0b7db27733aaa3c50c"
if (-not (Test-Path (Join-Path $BaseSrc "pyproject.toml"))) {
    throw "Base source not found at $BaseSrc. Extract the fork baseline there first (see tools/provision_base.py)."
}
Push-Location $BaseSrc
    uv build --wheel --out-dir $Wheels
Pop-Location
Push-Location "$Root\optional-mcps\mcp-cv-screen"
    uv build --wheel --out-dir $Wheels
Pop-Location
$baseWheel   = Get-ChildItem "$Wheels\hermes_agent-*.whl" | Select-Object -First 1
$cvScreenWh  = Get-ChildItem "$Wheels\mcp_cv_screen-*.whl" | Select-Object -First 1
Write-Host "  base:   $($baseWheel.Name)"
Write-Host "  cv-sc:  $($cvScreenWh.Name)"

# --- 3. collect offline wheel cache ------------------------------------------
Write-Step "Collect offline dependency wheels"
$Stage = Join-Path $Inst ".venv-stage"
Remove-Item -Recurse -Force $Stage -ErrorAction SilentlyContinue
uv venv --python 3.13 --seed $Stage
$py = Join-Path $Stage "Scripts\python.exe"
# base wheel + [mcp] extra; cv-screen --no-deps; light cv-screen deps (NO faster-whisper/torch)
uv pip install --python $py "$($baseWheel.FullName)[mcp]"
uv pip install --python $py --no-deps "$($cvScreenWh.FullName)"
uv pip install --python $py fastmcp pymupdf pillow httpx anyio

$freeze = uv pip freeze --python $py
# drop dev-only + the two local packages (they're copied, not downloaded)
$dev = @("debugpy","pytest","pytest-asyncio","ruff","ty","iniconfig","pluggy",
         "coverage","exceptiongroup","tomli","mypy","typing_extensions")
$req = $freeze | Where-Object {
    $line = $_
    $skip = $false
    foreach ($d in $dev) { if ($line -like "$d==*") { $skip = $true } }
    if ($line -like "hermes-agent==*") { $skip = $true }
    if ($line -like "mcp-cv-screen==*") { $skip = $true }
    -not $skip
}
$req | Set-Content -Encoding UTF8 (Join-Path $Inst "requirements-runtime.txt")
$StagePip = Join-Path $Stage "Scripts\pip.exe"
& $StagePip download -r (Join-Path $Inst "requirements-runtime.txt") -d $Wheels --no-deps
Write-Host "  wheels: $((Get-ChildItem $Wheels -Filter *.whl).Count) files"

# --- 4. portable Python ------------------------------------------------------
Write-Step "Provision portable Python 3.13"
uv python install 3.13
$pyRoot = (uv python find 3.13 | Split-Path)
Ensure-Dir (Join-Path $Payload "python")
Copy-Item -Recurse -Force "$pyRoot\*" (Join-Path $Payload "python")
Write-Host "  python: $((Get-ChildItem (Join-Path $Payload 'python') -Filter python.exe).FullName)"

# --- 5. bundle binaries + assets --------------------------------------------
Write-Step "Bundle MCPs / ffmpeg / web_dist / delta / skills"
Ensure-Dir (Join-Path $Payload "mcps")
Copy-Item (Join-Path $Rel "*.exe") (Join-Path $Payload "mcps") -Force
Remove-Item (Join-Path $Payload "mcps\*.pdb") -ErrorAction SilentlyContinue

Ensure-Dir (Join-Path $Payload "ffmpeg")
$ffSrc = Get-ChildItem "$env:LOCALAPPDATA\Microsoft\WinGet\Packages\Gyan.FFmpeg*\ffmpeg-*\bin" -Directory -ErrorAction SilentlyContinue | Select-Object -First 1
Copy-Item (Join-Path $ffSrc.FullName "ffmpeg.exe"),(Join-Path $ffSrc.FullName "ffprobe.exe") (Join-Path $Payload "ffmpeg") -Force

Ensure-Dir (Join-Path $Payload "web_dist")
Copy-Item -Recurse -Force "$Root\agent-core\hermes_cli\web_dist\*" (Join-Path $Payload "web_dist")

# delta: materialize the tracked office-agent agent-core files (81)
Ensure-Dir (Join-Path $Payload "delta\agent-core")
$delta = git -C $Root ls-files "agent-core"
foreach ($f in $delta) {
    $dst = Join-Path $Payload "delta\$($f -replace '/','\')"
    Ensure-Dir (Split-Path $dst)
    Copy-Item (Join-Path $Root ($f -replace '/','\')) $dst -Force
}

# skills: the 5 office-agent skill dirs
Ensure-Dir (Join-Path $Payload "skills")
foreach ($s in @("hr","marketing","reply-policy","audit","quota_budgeter")) {
    $src = Join-Path $Root "agent-core\skills\$s"
    if (Test-Path $src) { Copy-Item -Recurse -Force $src (Join-Path $Payload "skills") }
}

# provisioning machinery + scripts
Copy-Item "$Root\tools\provision_base.py" $Payload -Force
Copy-Item "$Root\tools\base_modified.txt","$Root\tools\base_added.txt" $Payload -Force
Copy-Item "$Inst\post-install.ps1","$Inst\launch.bat","$Inst\config-template.yaml" $Payload -Force

# --- 6. compile installer ----------------------------------------------------
Write-Step "Compile installer with Inno Setup"
$iscc = @(
    "$env:LOCALAPPDATA\Programs\Inno Setup 6\ISCC.exe",
    "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe",
    "$env:ProgramFiles\Inno Setup 6\ISCC.exe"
) | Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $iscc) { throw "ISCC.exe not found. Install: winget install JRSoftware.InnoSetup" }
& $iscc (Join-Path $Inst "OfficeAgent.iss")
Write-Host "`nDONE: $Dist\OfficeAgent-Setup.exe" -ForegroundColor Green