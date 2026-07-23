'use strict'

/**
 * Build the self-contained Hermes backend via PyInstaller.
 *
 * Produces a frozen `hermes` executable (onedir) at apps/desktop/build/
 * hermes-backend/ that the packaged Electron app spawns instead of cloning +
 * venving on first launch. The Electron main process picks it up via the
 * bundled-backend rung in resolveHermesBackend (electron/main.cjs).
 *
 * MUST run on the Windows host from the agent-core venv to produce a Windows
 * hermes.exe. (PyInstaller cannot cross-compile; a Linux venv yields a Linux
 * binary.) The build chain only invokes this for the Windows self-contained
 * build. Set HERMES_DESKTOP_SKIP_BACKEND=1 to bypass (e.g. for renderer-only
 * builds during dev).
 *
 * Also stages the optional-mcps manifests into build/mcp-manifests/ so the
 * backend's tools/mcp_sync can read them via HERMES_OPTIONAL_MCPS (set by
 * backend-env.cjs). The manifests are small YAML; they ride alongside the
 * backend bundle as a separate extraResource rather than inside the PyInstaller
 * archive so they can be inspected/edited without rebuilding.
 */

const fs = require('node:fs')
const path = require('node:path')
const { spawnSync } = require('node:child_process')

const APP_ROOT = path.resolve(__dirname, '..')
const REPO_ROOT = path.resolve(APP_ROOT, '..', '..')
const BUILD_ROOT = path.join(APP_ROOT, 'build')
const BACKEND_OUT = path.join(BUILD_ROOT, 'hermes-backend')
const PYI_WORK = path.join(BUILD_ROOT, '_pyi_work')
const MANIFESTS_OUT = path.join(BUILD_ROOT, 'mcp-manifests')

function venvPython() {
  // 1. Explicit override -- point at any python with hermes_agent installed.
  const override = process.env.HERMES_DESKTOP_VENV_PYTHON
  if (override && fs.existsSync(override)) return override
  // 2. agent-core/.venv (the convention used by install.ps1 / install.sh).
  if (process.platform === 'win32') {
    const p = path.join(REPO_ROOT, '.venv', 'Scripts', 'python.exe')
    if (fs.existsSync(p)) return p
  } else {
    const p = path.join(REPO_ROOT, '.venv', 'bin', 'python')
    if (fs.existsSync(p)) return p
  }
  // 3. `python` on PATH (last resort; must have hermes_agent importable).
  const which = spawnSync(
    process.platform === 'win32' ? 'where' : 'which',
    [process.platform === 'win32' ? 'python' : 'python3'],
    { encoding: 'utf8' }
  )
  if (which.status === 0) {
    const candidate = (which.stdout || '').split(/\r?\n/)[0].trim()
    if (candidate && fs.existsSync(candidate)) return candidate
  }
  return null
}

function rmrf(p) {
  if (!p) return
  try {
    fs.rmSync(p, { recursive: true, force: true })
  } catch {
    /* ignore */
  }
}

function run(cmd, args, opts = {}) {
  console.log(`[build-hermes-backend] $ ${cmd} ${args.join(' ')}`)
  const res = spawnSync(cmd, args, { stdio: 'inherit', ...opts })
  if (res.status !== 0) {
    throw new Error(`[build-hermes-backend] command failed (exit ${res.status}): ${cmd} ${args.join(' ')}`)
  }
}

function ensurePyinstaller(python) {
  // Check via -c import; install into the venv if missing.
  const probe = spawnSync(python, ['-c', 'import PyInstaller'], { stdio: 'ignore' })
  if (probe.status === 0) return
  console.log('[build-hermes-backend] PyInstaller not found in venv; installing...')
  run(python, ['-m', 'pip', 'install', '--disable-pip-version-check', 'pyinstaller'])
}

function stageManifests() {
  rmrf(MANIFESTS_OUT)
  fs.mkdirSync(MANIFESTS_OUT, { recursive: true })
  const src = path.join(REPO_ROOT, 'optional-mcps')
  if (!fs.existsSync(src)) {
    console.log('[build-hermes-backend] no optional-mcps/ at repo root; skipping manifest staging')
    return
  }
  let count = 0
  for (const entry of fs.readdirSync(src, { withFileTypes: true })) {
    if (!entry.isDirectory()) continue
    const manifest = path.join(src, entry.name, 'manifest.yaml')
    if (!fs.existsSync(manifest)) continue
    const destDir = path.join(MANIFESTS_OUT, entry.name)
    fs.mkdirSync(destDir, { recursive: true })
    fs.copyFileSync(manifest, path.join(destDir, 'manifest.yaml'))
    count++
  }
  console.log(`[build-hermes-backend] staged ${count} MCP manifest(s) -> ${MANIFESTS_OUT}`)
}

function buildBackend() {
  const python = venvPython()
  if (!python) {
    throw new Error(
      `[build-hermes-backend] no venv python found at ${REPO_ROOT}/.venv. ` +
        `Create it first: cd ${REPO_ROOT} && uv venv && uv pip install -e .`
    )
  }
  ensurePyinstaller(python)

  // Fresh output dirs so stale artifacts don't survive.
  rmrf(BACKEND_OUT)
  rmrf(PYI_WORK)

  // Entry: hermes_cli/main.py has `if __name__ == "__main__": main()` so it
  // doubles as the console-script entry (hermes = hermes_cli.main:main).
  const entry = path.join(REPO_ROOT, 'hermes_cli', 'main.py')

  const args = [
    '-m', 'PyInstaller',
    '--noconfirm',
    '--onedir',
    '--name', 'hermes',
    '--distpath', BACKEND_OUT,
    '--workpath', PYI_WORK,
    '--specpath', PYI_WORK,
    // Collect the heavy first-party packages so dynamic imports survive
    // freezing. hermes_cli is large with many submodules; --collect-submodules
    // pulls them all. --collect-all hermes_agent grabs data files too.
    '--collect-all', 'hermes_agent',
    '--collect-submodules', 'hermes_cli',
    '--collect-submodules', 'agent',
    '--collect-submodules', 'tools',
    '--collect-submodules', 'gateway',
    '--collect-submodules', 'providers',
    '--collect-submodules', 'acp_adapter',
    // run_agent.py is a top-level py-module, not a package — collect it.
    '--hidden-import', 'run_agent',
    entry
  ]

  run(python, args, { cwd: REPO_ROOT })

  const exe = path.join(BACKEND_OUT, process.platform === 'win32' ? 'hermes.exe' : 'hermes')
  if (!fs.existsSync(exe)) {
    throw new Error(`[build-hermes-backend] expected output not found: ${exe}`)
  }
  console.log(`[build-hermes-backend] built ${exe}`)

  // Smoke-test the frozen CLI so a broken bundle (missing hidden import) fails
  // the build here rather than at first launch on a clean Windows box.
  const smoke = spawnSync(exe, ['--version'], { stdio: 'pipe', encoding: 'utf8' })
  if (smoke.status !== 0) {
    console.error(`[build-hermes-backend] smoke test FAILED (exit ${smoke.status})`)
    console.error(smoke.stderr || smoke.stdout || '(no output)')
    throw new Error('[build-hermes-backend] frozen hermes --version smoke test failed')
  }
  console.log(`[build-hermes-backend] smoke test OK: ${(smoke.stdout || '').trim().split('\n')[0]}`)
}

function main() {
  if (process.env.HERMES_DESKTOP_SKIP_BACKEND === '1') {
    console.log('[build-hermes-backend] HERMES_DESKTOP_SKIP_BACKEND=1; skipping')
    return
  }
  if (process.platform !== 'win32' && process.env.HERMES_DESKTOP_TARGET_PLATFORM !== 'win32') {
    console.warn(
      '[build-hermes-backend] WARNING: not running on Windows. PyInstaller will ' +
        'produce a host-platform binary, not a Windows hermes.exe. Run this on ' +
        'the Windows host for the self-contained Windows build.'
    )
  }
  stageManifests()
  buildBackend()
  console.log('[build-hermes-backend] done')
}

try {
  main()
} catch (err) {
  console.error(`[build-hermes-backend] FAILED: ${err.message}`)
  process.exit(1)
}