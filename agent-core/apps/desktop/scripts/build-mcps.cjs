'use strict'

/**
 * Build bundled MCP server executables for the self-contained desktop app.
 *
 * Today only n8n is a local stdio server (linear and unreal-engine are remote
 * HTTP URLs -- nothing to build, they are registered as config entries by
 * tools/mcp_sync at first launch). n8n's manifest declares a git-clone install
 * (optional-mcps/n8n/manifest.yaml); instead of cloning at install time on the
 * user's machine, we clone + PyInstaller-freeze its server.py here and ship the
 * resulting n8n.exe under resources/mcps/. mcp_sync registers it with an
 * absolute `command` path so tools/mcp_tool._resolve_stdio_command spawns it
 * directly (no PATH lookup, no clone on the host).
 *
 * MUST run on the Windows host (PyInstaller cannot cross-compile). n8n build
 * failure is NON-FATAL: linear and unreal-engine still ship auto-registered;
 * the user can `hermes mcp install n8n` later. Set HERMES_DESKTOP_SKIP_MCPS=1
 * to skip entirely (e.g. renderer-only dev builds).
 */

const fs = require('node:fs')
const os = require('node:os')
const path = require('node:path')
const { spawnSync } = require('node:child_process')

const APP_ROOT = path.resolve(__dirname, '..')
const REPO_ROOT = path.resolve(APP_ROOT, '..', '..')
const BUILD_ROOT = path.join(APP_ROOT, 'build')
const MCPS_OUT = path.join(BUILD_ROOT, 'mcps')

function rmrf(p) {
  if (!p) return
  try {
    fs.rmSync(p, { recursive: true, force: true })
  } catch {
    /* ignore */
  }
}

function venvPython() {
  const override = process.env.HERMES_DESKTOP_VENV_PYTHON
  if (override && fs.existsSync(override)) return override
  if (process.platform === 'win32') {
    const p = path.join(REPO_ROOT, '.venv', 'Scripts', 'python.exe')
    if (fs.existsSync(p)) return p
  } else {
    const p = path.join(REPO_ROOT, '.venv', 'bin', 'python')
    if (fs.existsSync(p)) return p
  }
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

function run(cmd, args, opts = {}) {
  console.log(`[build-mcps] $ ${cmd} ${args.join(' ')}`)
  const res = spawnSync(cmd, args, { stdio: 'inherit', ...opts })
  if (res.status !== 0) {
    throw new Error(`[build-mcps] command failed (exit ${res.status}): ${cmd} ${args.join(' ')}`)
  }
}

// Minimal manifest reader -- extract install.url and install.ref without a YAML
// dependency. The manifest is small and these fields are single-line scalars.
function readN8nManifest() {
  const manifest = path.join(REPO_ROOT, 'optional-mcps', 'n8n', 'manifest.yaml')
  if (!fs.existsSync(manifest)) return null
  const text = fs.readFileSync(manifest, 'utf8')
  const url = /^url:\s*(.+)$/m.exec(text)
  const ref = /^ref:\s*(.+)$/m.exec(text)
  if (!url || !ref) return null
  return { url: url[1].trim(), ref: ref[1].trim() }
}

function buildN8n() {
  const spec = readN8nManifest()
  if (!spec) {
    console.log('[build-mcps] no n8n manifest found; skipping n8n')
    return
  }
  const bootstrapPython = venvPython()
  if (!bootstrapPython) {
    throw new Error('[build-mcps] agent-core venv python not found; cannot bootstrap n8n venv')
  }

  const work = fs.mkdtempSync(path.join(os.tmpdir(), 'hermes-n8n-'))
  try {
    const cloneDir = path.join(work, 'repo')
    run('git', ['clone', '--depth', '1', '--branch', spec.ref, spec.url, cloneDir])
    // n8n manifest's bootstrap uses python3 -m venv; use the agent-core venv
    // python to create the n8n venv so we don't depend on a system python3.
    const n8nVenv = path.join(cloneDir, '.venv')
    run(bootstrapPython, ['-m', 'venv', n8nVenv])
    const n8nPython =
      process.platform === 'win32'
        ? path.join(n8nVenv, 'Scripts', 'python.exe')
        : path.join(n8nVenv, 'bin', 'python')

    const reqs = path.join(cloneDir, 'requirements.txt')
    if (fs.existsSync(reqs)) {
      run(n8nPython, ['-m', 'pip', 'install', '--disable-pip-version-check', '-r', reqs])
    }
    // PyInstaller must run in the env that has n8n's deps.
    run(n8nPython, ['-m', 'pip', 'install', '--disable-pip-version-check', 'pyinstaller'])

    const serverPy = path.join(cloneDir, 'server.py')
    if (!fs.existsSync(serverPy)) {
      throw new Error(`[build-mcps] n8n server.py not found at ${serverPy}`)
    }
    const dist = path.join(work, 'dist')
    const pyiWork = path.join(work, 'pyi-work')
    run(
      n8nPython,
      [
        '-m', 'PyInstaller',
        '--noconfirm',
        '--onedir',
        '--name', 'n8n',
        '--distpath', dist,
        '--workpath', pyiWork,
        '--specpath', pyiWork,
        serverPy
      ],
      { cwd: cloneDir }
    )

    const builtExe = path.join(dist, process.platform === 'win32' ? 'n8n.exe' : 'n8n')
    if (!fs.existsSync(builtExe)) {
      throw new Error(`[build-mcps] expected n8n output not found: ${builtExe}`)
    }
    fs.mkdirSync(MCPS_OUT, { recursive: true })
    // Copy the whole onedir bundle (exe + _internal/) so DLLs/deps ship too.
    const destDir = path.join(MCPS_OUT, 'n8n')
    rmrf(destDir)
    fs.cpSync(dist, destDir, { recursive: true })
    console.log(`[build-mcps] n8n staged at ${path.join(destDir, process.platform === 'win32' ? 'n8n.exe' : 'n8n')}`)
  } finally {
    rmrf(work)
  }
}

function main() {
  if (process.env.HERMES_DESKTOP_SKIP_MCPS === '1') {
    console.log('[build-mcps] HERMES_DESKTOP_SKIP_MCPS=1; skipping')
    return
  }
  rmrf(MCPS_OUT)
  fs.mkdirSync(MCPS_OUT, { recursive: true })

  // linear + unreal-engine are remote HTTP -- no build step. n8n is the only
  // local stdio server. Its build is best-effort so a third-party-repo failure
  // doesn't block the rest of the self-contained build.
  try {
    buildN8n()
  } catch (err) {
    console.warn(`[build-mcps] n8n build FAILED (non-fatal): ${err.message}`)
    console.warn('[build-mcps] n8n will not be bundled; linear + unreal-engine still ship auto-registered.')
    console.warn('[build-mcps] users can install n8n later via `hermes mcp install n8n`.')
  }
  console.log('[build-mcps] done')
}

try {
  main()
} catch (err) {
  console.error(`[build-mcps] FAILED: ${err.message}`)
  process.exit(1)
}