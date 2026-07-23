"use strict"

// Resolve electronDist at runtime (#38673, #47917): electron-builder 26.8.x can
// re-unpack a broken Electron.app; reusing the installed dist dodges that.
// npm workspace hoisting is non-deterministic — require.resolve finds electron
// wherever it landed. Dist present → -c.electronDist=<abs>/dist; absent → let
// electron-builder fetch via @electron/get (electronVersion + ELECTRON_MIRROR).

const fs = require("node:fs")
const path = require("node:path")
const { spawnSync } = require("node:child_process")

function electronDistDir() {
  try {
    return path.join(path.dirname(require.resolve("electron/package.json")), "dist")
  } catch {
    return null
  }
}

function distBinary(dist) {
  if (process.platform === "darwin") {
    return path.join(dist, "Electron.app", "Contents", "MacOS", "Electron")
  }
  if (process.platform === "win32") {
    return path.join(dist, "electron.exe")
  }
  return path.join(dist, "electron")
}

function electronBuilderCli() {
  const pkgJson = require.resolve("electron-builder/package.json")
  const bin = require(pkgJson).bin
  const rel = typeof bin === "string" ? bin : bin["electron-builder"]
  return path.join(path.dirname(pkgJson), rel)
}

const dist = electronDistDir()
const cliArgs = process.argv.slice(2)
// Detect target platform from CLI flags (--win / --mac / --linux or bare
// tokens). Default to host platform when unspecified.
const targetWin = cliArgs.some((a) => /--win\b|win32\b/i.test(a))
const targetMac = cliArgs.some((a) => /--mac\b/i.test(a))
const targetLinux = cliArgs.some((a) => /--linux\b/i.test(a))
const targetPlatform = targetWin
  ? "win32"
  : targetMac
    ? "darwin"
    : targetLinux
      ? "linux"
      : process.platform
const args = []
// Only reuse the locally installed electron dist when its platform matches
// the build target. Cross-building (e.g. --win from Linux) must fetch the
// target-platform electron via @electron/get — the local Linux dist has no
// electron.exe, so forcing -c.electronDist=<linux dist> yields ENOENT on the
// rename to Hermes.exe.
if (dist && targetPlatform === process.platform && fs.existsSync(distBinary(dist))) {
  args.push(`-c.electronDist=${dist}`)
} else {
  console.warn(
    `[run-electron-builder] ${
      targetPlatform === process.platform
        ? "no local electron dist;"
        : `cross-build (${process.platform} -> ${targetPlatform}); local dist wrong platform;`
    } electron-builder will fetch via @electron/get (electronVersion + ELECTRON_MIRROR).`
  )
}
args.push(...cliArgs)

const result = spawnSync(process.execPath, [electronBuilderCli(), ...args], {
  stdio: "inherit",
})
if (result.error) {
  console.error(`[run-electron-builder] spawn failed: ${result.error.message}`)
  process.exit(1)
}
process.exit(result.status == null ? 1 : result.status)
