# ============================================================================
# office-agent post-install: build venv, offline pip, overlay delta, write
# config, copy skills, set env. Run by Inno Setup [Run] (elevated).
#   powershell -ExecutionPolicy Bypass -File post-install.ps1 -InstallDir "C:\Program Files\Office Agent"
# ============================================================================
[CmdletBinding()]
param(
    [Parameter(Mandatory=$true)][string]$InstallDir,
    [string]$HermesHome,
    [switch]$DryRun
)
$ErrorActionPreference = "Stop"
function W-Step($m){ Write-Host "`n==> $m" -ForegroundColor Cyan }

if (-not $HermesHome) { $HermesHome = Join-Path $env:LOCALAPPDATA "hermes" }
$Venv       = Join-Path $HermesHome "venv"
$VenvPy     = Join-Path $Venv "Scripts\python.exe"
$VenvPip    = Join-Path $Venv "Scripts\pip.exe"
$Wheels     = Join-Path $InstallDir "wheels"

# --- 1. HERMES_HOME dirs -----------------------------------------------------
W-Step "Create HERMES_HOME at $HermesHome"
foreach ($d in @("data","skills","cache","logs","sessions","work")) {
    New-Item -ItemType Directory -Force -Path (Join-Path $HermesHome $d) | Out-Null
}

# --- 2. venv from bundled portable Python -----------------------------------
W-Step "Create venv"
$PortablePy = Join-Path $InstallDir "python\python.exe"
if (-not (Test-Path $PortablePy)) { throw "Portable python not found: $PortablePy" }
& $PortablePy -m venv $Venv
if (-not (Test-Path $VenvPip)) { & $VenvPy -m ensurepip --upgrade; & $VenvPy -m pip install --upgrade pip }

# --- 3. offline install (no network) ----------------------------------------
W-Step "Offline pip install: hermes_agent[mcp] + cv-screen (light)"
& $VenvPip install --no-index --find-links $Wheels "hermes_agent[mcp]"
& $VenvPip install --no-index --find-links $Wheels --no-deps (Get-ChildItem "$Wheels\mcp_cv_screen-*.whl" | Select-Object -First 1).FullName
& $VenvPip install --no-index --find-links $Wheels fastmcp pymupdf pillow httpx anyio

# --- 4. overlay the 81-file office-agent delta onto site-packages -----------
W-Step "Overlay office-agent delta"
& $VenvPy (Join-Path $InstallDir "provision_base.py") install `
    --base-wheel (Get-ChildItem "$Wheels\hermes_agent-*.whl" | Select-Object -First 1).FullName `
    --venv $Venv --delta-root (Join-Path $InstallDir "delta")

# --- 5. copy office-agent skills into the active skills dir -----------------
W-Step "Copy office-agent skills -> $HermesHome\skills"
$SkillsSrc = Join-Path $InstallDir "skills"
if (Test-Path $SkillsSrc) {
    Copy-Item -Recurse -Force "$SkillsSrc\*" (Join-Path $HermesHome "skills")
}

# --- 6. write config.yaml (all MCPs pre-registered, enabled) ----------------
W-Step "Write config.yaml"
$Tpl = Get-Content (Join-Path $InstallDir "config-template.yaml") -Raw
$Cfg = $Tpl `
    -replace '\{\{INSTALL_DIR\}\}',  ($InstallDir -replace '\\','\\') `
    -replace '\{\{HERMES_HOME\}\}',  ($HermesHome   -replace '\\','\\') `
    -replace '\{\{VENV_PY\}\}',      ($VenvPy       -replace '\\','\\')
Set-Content -Encoding UTF8 -Path (Join-Path $HermesHome "config.yaml") -Value $Cfg

# --- 7. user env vars --------------------------------------------------------
W-Step "Set user env (HERMES_HOME, PATH += ffmpeg)"
if ($DryRun) {
    Write-Host "  (dry-run: skipping real env-var writes)"
} else {
    [Environment]::SetEnvironmentVariable("HERMES_HOME", $HermesHome, "User")
    $ffDir = Join-Path $InstallDir "ffmpeg"
    $userPath = [Environment]::GetEnvironmentVariable("Path","User")
    if ($userPath -notlike "*$ffDir*") {
        [Environment]::SetEnvironmentVariable("Path", ($userPath.TrimEnd(';') + ";$ffDir"), "User")
    }
}

W-Step "Done. Shortcut runs: hermes dashboard --skip-build  (http://127.0.0.1:9119/)"