'use strict'

/**
 * Stage native (non-pip) runtime binaries -- ffmpeg.exe and rg.exe -- for the
 * self-contained Windows desktop build.
 *
 * PyInstaller bundles the Python runtime + hermes_agent + pip deps, but it
 * does NOT include ffmpeg (TTS / mcp-video-edit) or ripgrep (fast file search),
 * which install.ps1 otherwise installs via winget (scripts/install.ps1:1117,
 * 1123). The self-contained build has no winget step, so we ship the Windows
 * binaries ourselves and backend-env.cjs prepends resources/native-bin to the
 * backend's PATH.
 *
 * Runs as part of `npm run build` (gated to the bundle build path). Idempotent:
 * re-downloads only if the staged exe is missing. Downloads are cached under
 * build/native-bin-cache/ so repeated builds don't re-fetch.
 *
 * No-op when the target is not win32 (macOS/Linux bundled builds are not the
 * focus; the host install provides ffmpeg/rg there). Set
 * HERMES_DESKTOP_TARGET_PLATFORM=win32 to force Windows bins on a cross-build.
 */

const fs = require('node:fs')
const path = require('node:path')
const { spawnSync } = require('node:child_process')
const https = require('node:https')

const APP_ROOT = path.resolve(__dirname, '..')
const STAGE_ROOT = path.join(APP_ROOT, 'build', 'native-bin')
const CACHE_ROOT = path.join(APP_ROOT, 'build', 'native-bin-cache')

// Pinned download URLs. Pinned (not "latest") so the build is reproducible.
// ffmpeg: gyan.dev essentials build (Windows x64). rg: BurntSushi MSVC build.
const ASSETS = {
  win32: [
    {
      name: 'ffmpeg',
      url: 'https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip',
      cacheZip: 'ffmpeg-essentials.zip',
      // Inside the zip: ffmpeg-*-essentials_build/bin/ffmpeg.exe
      findExe: (extractedRoot) => findFile(extractedRoot, 'ffmpeg.exe')
    },
    {
      name: 'rg',
      url: 'https://github.com/BurntSushi/ripgrep/releases/download/14.0.3/ripgrep-14.0.3-x86_64-pc-windows-msvc.zip',
      cacheZip: 'rg.zip',
      findExe: (extractedRoot) => findFile(extractedRoot, 'rg.exe')
    }
  ]
}

function targetPlatform() {
  const forced = process.env.HERMES_DESKTOP_TARGET_PLATFORM
  if (forced) return forced
  return process.platform
}

function rmrf(p) {
  if (!p) return
  try {
    fs.rmSync(p, { recursive: true, force: true })
  } catch {
    /* ignore */
  }
}

function findFile(root, filename) {
  try {
    for (const entry of fs.readdirSync(root, { withFileTypes: true })) {
      const full = path.join(root, entry.name)
      if (entry.isFile() && entry.name.toLowerCase() === filename.toLowerCase()) {
        return full
      }
      if (entry.isDirectory()) {
        const found = findFile(full, filename)
        if (found) return found
      }
    }
  } catch {
    /* ignore */
  }
  return null
}

function download(url, dest) {
  return new Promise((resolve, reject) => {
    const file = fs.createWriteStream(dest)
    const req = (currentUrl) => {
      https
        .get(currentUrl, (res) => {
          // Follow redirects (github releases → objects storage).
          if (res.statusCode >= 300 && res.statusCode < 400 && res.headers.location) {
            res.resume()
            return req(res.headers.location)
          }
          if (res.statusCode !== 200) {
            res.resume()
            return reject(new Error(`HTTP ${res.statusCode} for ${currentUrl}`))
          }
          res.pipe(file)
          file.on('finish', () => file.close(() => resolve(dest)))
        })
        .on('error', (err) => {
          file.close()
          rmrf(dest)
          reject(err)
        })
    }
    req(url)
  })
}

function extractZip(zip, dest) {
  // Windows 10 1803+ ships tar.exe which handles .zip. Fallback: PowerShell
  // Expand-Archive. (No npm zip dep to keep the build lean.)
  fs.mkdirSync(dest, { recursive: true })
  const tar = spawnSync('tar', ['-xf', zip, '-C', dest], { stdio: 'ignore' })
  if (tar.status === 0) return true
  const ps = spawnSync(
    'powershell',
    ['-NoProfile', '-Command', `Expand-Archive -LiteralPath '${zip}' -DestinationPath '${dest}' -Force`],
    { stdio: 'ignore' }
  )
  return ps.status === 0
}

async function stageOne(asset) {
  const stagedExe = path.join(STAGE_ROOT, process.platform === 'win32' ? `${asset.name}.exe` : asset.name)
  if (fs.existsSync(stagedExe)) {
    console.log(`[stage-native-bin] ${asset.name}: already staged`)
    return
  }
  fs.mkdirSync(CACHE_ROOT, { recursive: true })
  const zipPath = path.join(CACHE_ROOT, asset.cacheZip)
  if (!fs.existsSync(zipPath)) {
    console.log(`[stage-native-bin] ${asset.name}: downloading ${asset.url}`)
    await download(asset.url, zipPath)
  }
  const extractDir = path.join(CACHE_ROOT, `${asset.name}-extracted`)
  rmrf(extractDir)
  if (!extractZip(zipPath, extractDir)) {
    throw new Error(`[stage-native-bin] ${asset.name}: failed to extract ${zipPath}`)
  }
  const exe = asset.findExe(extractDir)
  if (!exe) {
    throw new Error(`[stage-native-bin] ${asset.name}: ${asset.findExe.name} not found in ${zipPath}`)
  }
  fs.mkdirSync(STAGE_ROOT, { recursive: true })
  fs.copyFileSync(exe, stagedExe)
  // Best-effort +x for non-Windows staged bins.
  if (process.platform !== 'win32') {
    try {
      fs.chmodSync(stagedExe, 0o755)
    } catch {
      /* ignore */
    }
  }
  console.log(`[stage-native-bin] ${asset.name}: staged at ${stagedExe}`)
}

async function main() {
  const platform = targetPlatform()
  const assets = ASSETS[platform]
  if (!assets || assets.length === 0) {
    console.log(`[stage-native-bin] target platform '${platform}' has no native bins to stage; skipping`)
    return
  }
  // Fresh stage dir so stale bins don't survive a URL/version bump.
  rmrf(STAGE_ROOT)
  fs.mkdirSync(STAGE_ROOT, { recursive: true })
  for (const asset of assets) {
    await stageOne(asset)
  }
  console.log(`[stage-native-bin] done -> ${STAGE_ROOT}`)
}

main().catch((err) => {
  console.error(`[stage-native-bin] FAILED: ${err.message}`)
  process.exit(1)
})