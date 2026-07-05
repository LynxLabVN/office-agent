# Phase 6 — Tauri migration & full reskin

**Goal:** Replace the Electron desktop app with a Tauri shell, port all native
features, and reskin both the web dashboard and the desktop app with the Open
Design system from `.docs/oa-index.html`.
**Depends on:** Phase 4 (UI exists) + Phase 5 (hardening baseline).
**Duration:** ~10–12 weeks (critical path 7 weeks).
**Parallelizable with:** nothing (this is the final track).

Read [`00-overview.md`](./00-overview.md) first for workspace structure and
conventions. Read [`phase-4-opendesign-prompt.md`](./phase-4-opendesign-prompt.md)
for the design-system reference (8 themes, CSS variables, component specs).

---

## Assumptions (confirmed before starting)

| # | Question | Decision |
|---|----------|----------|
| A1 | PTY approach | Rust `portable-pty` crate (no Node sidecar) |
| A2 | Pet overlay | Must-have, lands last in Tier 3 |
| A3 | VSCode marketplace themes | Must-have, lands last in Tier 3 |
| A4 | `apps/desktop` handling | Replace in-place at Phase 6.7 (keep during transition) |
| A5 | First sub-phase to execute | Phase 6.0 (PTY spike) immediately |

---

## Architecture summary

The existing Electron desktop app (`apps/desktop/`) is a separate React app
from the web dashboard (`agent-core/web/`). Both will be reskinned with a
shared design-system package. The desktop app moves from Electron to Tauri.

**Key insight:** the entire native surface is one bridge — `window.hermesDesktop`
(exposed by `electron/preload.cjs`, ~80 methods). The React renderer calls
through this bridge and never touches Electron directly. By building a
Tauri-backed shim with the same shape, the **React code doesn't change** —
only the bridge implementation swaps from Electron IPC to Tauri commands.

**Chat does NOT use PTY.** The desktop chat (`@assistant-ui/react`) talks to
the gateway over WebSocket/JSON-RPC via `@hermes/shared`'s
`JsonRpcGatewayClient`. PTY is only used by the terminal sidebar
(`window.hermesDesktop.terminal`).

---

## Sub-phase overview

| Sub-phase | Goal | Duration | Depends on | Parallel with |
| --------- | ---- | -------- | ---------- | ------------- |
| 6.0 | PTY risk spike | 1 week | nothing | — (gate) |
| 6.1 | Shared design system package | 2 weeks | 6.0 gate | 6.2 (after week 1) |
| 6.2 | Tauri shell + `hermesDesktop` bridge | 2 weeks | 6.0 | 6.1 (after week 1) |
| 6.3 | Port all native features | 3–4 weeks | 6.2 | — |
| 6.4 | Reskin desktop UI | 2 weeks | 6.1 | 6.5 |
| 6.5 | Reskin web dashboard | 2–3 weeks | 6.1 | 6.4 |
| 6.6 | Backend hardening | 1 week | 6.1 | 6.4, 6.5 |
| 6.7 | Packaging & cleanup | 1 week | 6.3, 6.4, 6.5 | — |

**Critical path:** 6.0 → 6.2 → 6.3 → 6.7 = **7 weeks**
**Total with parallelism:** **~10–12 weeks**

```
Week 1:   6.0 (PTY spike) ──────────── GATE
Week 2-3: 6.1 (design system) ────────┐
Week 2-3: 6.2 (Tauri shell+bridge) ───┤
Week 4-6: 6.3 (native features) ──────┘
Week 4-5: 6.4 (desktop reskin) ───────┐ parallel
Week 4-6: 6.5 (web reskin) ───────────┘ parallel
Week 4+:  6.6 (backend hardening) ──── anytime after 6.1
Week 7:   6.7 (packaging & cleanup)
```

---

## 6.0 — PTY risk spike

**Goal:** Prove a Rust-backed PTY streams to xterm.js inside a Tauri webview.
Gate the entire migration on this.

**Duration:** 1 week
**Depends on:** nothing
**Blocks:** everything downstream

### Tasks

1. Scaffold minimal Tauri app at `agent-core/apps/desktop-tauri-spike/`
   (throwaway).
2. Add `portable-pty` crate to `Cargo.toml`.
3. Implement Tauri commands: `terminal_start`, `terminal_write`,
   `terminal_resize`, `terminal_dispose`.
4. Emit `terminal:data` and `terminal:exit` events to the webview.
5. Minimal HTML page mounting xterm.js, calling the commands, rendering
   streamed output.
6. Test on macOS, Linux, Windows (WSL).
7. **Gate decision:**
   - If works → proceed to 6.1 + 6.2.
   - If fails → fallback: Node.js sidecar running `node-pty`, communicate
     via stdio.

### Files (new)

- `apps/desktop-tauri-spike/Cargo.toml`
- `apps/desktop-tauri-spike/src/main.rs`
- `apps/desktop-tauri-spike/src/terminal.rs`
- `apps/desktop-tauri-spike/index.html`
- `apps/desktop-tauri-spike/tauri.conf.json`

### Verify

- [ ] Type `ls` in the xterm pane → see directory listing.
- [ ] Resize window → PTY receives SIGWINCH equivalent.
- [ ] Close tab → PTY disposed, no leak.
- [ ] Works on all 3 OSes (macOS, Linux, Windows/WSL).

---

## 6.1 — Shared design system package

**Goal:** One design-system package both `web/` and `apps/desktop/` import.
Replaces the Open Design HTML reference's tokens with consumable React +
Tailwind.

**Duration:** 2 weeks
**Depends on:** 6.0 gate passes
**Can parallel with:** 6.2 (after first week)

### Tasks

1. Create `agent-core/packages/design-system/` workspace package.
2. Port the 8 theme palettes + light/dark from `.docs/oa-index.html` into CSS
   variables:
   - `hermes-teal`, `hermes-teal-large`, `nous-blue`, `midnight`, `ember`,
     `mono`, `cyberpunk`, `rose`
3. Build `ThemeProvider` that sets `data-theme` / `data-mode` on `<html>` and
   persists to localStorage.
4. Build primitive components matching the reference's `.btn`, `.panel`,
   `.input`, `.nav-link`, `.topbar`, `.sidebar` classes:
   - `Button` (primary / ghost / sm / icon variants)
   - `Card` / `CardHeader` / `CardContent` / `CardTitle`
   - `Input` / `Label` / `Textarea` / `Select`
   - `Badge` / `Avatar` / `Spinner` / `Skeleton`
   - `Modal` (centered, backdrop blur, Esc / close-on-outside)
   - `Table` (sticky header, hover, 44px rows)
   - `NavLink` / `NavGroup` / `TopBar` / `Sidebar`
   - `Toast`
5. Tailwind v4 preset exporting the CSS-variable-based color tokens.
6. Font / density / radius controls wired to `--theme-*` variables.
7. Storybook-style demo page for visual verification.

### Files (new)

- `packages/design-system/package.json`
- `packages/design-system/src/index.ts`
- `packages/design-system/src/themes/tokens.css`
- `packages/design-system/src/themes/presets.ts`
- `packages/design-system/src/themes/provider.tsx`
- `packages/design-system/src/components/*.tsx`
- `packages/design-system/tailwind.preset.ts`
- `packages/design-system/demo/index.html`

### Files (modified)

- `agent-core/web/package.json` — add `@office-agent/design-system` dep
- `agent-core/apps/desktop/package.json` — add same dep

### Verify

- [ ] `npm run build` in `packages/design-system/` succeeds.
- [ ] Demo page renders all 8 themes with live switching.
- [ ] Both `web/` and `apps/desktop/` can import `{ Button }` from the package.
- [ ] Light/dark toggle works without reload.

---

## 6.2 — Tauri shell + `hermesDesktop` bridge

**Goal:** A Tauri app that launches, spawns the backend, and exposes
`window.hermesDesktop` with the same shape as `electron/preload.cjs`. The
existing React code runs unmodified.

**Duration:** 2 weeks
**Depends on:** 6.0 gate
**Can parallel with:** 6.1 (after week 1)

### Tasks

1. Create `agent-core/apps/desktop-tauri/` (the real app, not the spike).
2. Copy `apps/desktop/src/` (React renderer) into `apps/desktop-tauri/src/`.
3. Configure Tauri to serve the built renderer (`dist/`).
4. Implement Rust backend-spawn pipeline:
   - Discover `hermes` binary (PATH, packaged, managed install).
   - Spawn `hermes serve` (or `hermes web` fallback for older runtimes).
   - Build desktop backend env (port `backend-env.cjs` logic → Rust).
   - Wait for `HERMES_DASHBOARD_READY` on stdout.
   - Read bound port, exchange auth token.
5. Implement `window.hermesDesktop` shim:
   - `src/preload.ts` injects `window.hermesDesktop` backed by
     `@tauri-apps/api` `invoke` + `listen`.
   - Shape matches `electron/preload.cjs` exactly (same method names, same
     signatures).
   - Methods not yet implemented throw `not_implemented` so the renderer
     fails gracefully.
6. Implement Tier-1 commands (see 6.3) so the app boots end-to-end:
   - `getConnection` / `touchBackend` / `getGatewayWsUrl`
   - `getConnectionConfig` / `saveConnectionConfig` / `applyConnectionConfig`
   - `profile.get` / `profile.set`
   - `onDeepLink` / `signalDeepLinkReady`
   - `terminal.*` (from 6.0 spike, ported over)
7. Window management: main window, state persistence, min size.
8. Deep-link protocol (`hermes://`).

### Files (new)

- `apps/desktop-tauri/Cargo.toml`
- `apps/desktop-tauri/tauri.conf.json`
- `apps/desktop-tauri/src/main.rs`
- `apps/desktop-tauri/src/backend.rs`
- `apps/desktop-tauri/src/connection.rs`
- `apps/desktop-tauri/src/terminal.rs`
- `apps/desktop-tauri/src/bridge.rs`
- `apps/desktop-tauri/src/preload.ts`
- `apps/desktop-tauri/src/index.html`

### Files (copied)

- `apps/desktop/src/**` → `apps/desktop-tauri/src/**` (renderer, unmodified)

### Verify

- [ ] `cargo tauri dev` launches the app.
- [ ] Backend spawns, dashboard loads in the webview.
- [ ] Chat connects over WebSocket (no PTY needed for chat).
- [ ] Terminal sidebar opens a shell (PTY works).
- [ ] Deep links resolve.
- [ ] No Electron process running.

---

## 6.3 — Port all native features

**Goal:** Every method on `window.hermesDesktop` works under Tauri. No method
throws `not_implemented`.

**Duration:** 3–4 weeks
**Depends on:** 6.2

### Tier 1 — Critical path (week 1)

Already done in 6.2 (terminal, connection, profile, deep links). Verify and
harden.

### Tier 2 — Core features (week 2)

**File system:**

| Method | Rust implementation |
|--------|---------------------|
| `readFileDataUrl` | `std::fs` + base64 encode |
| `readFileText` | `std::fs::read_to_string` |
| `readDir` | `std::fs::read_dir` |
| `writeTextFile` | `std::fs::write` |
| `renamePath` | `std::fs::rename` |
| `trashPath` | platform trash crate |
| `revealPath` | `open` / `xdg-open` / `explorer` |
| `getPathForFile` | Tauri webview file path mapping |
| `sanitizeWorkspaceCwd` | path validation logic |

**Git operations:**

| Method | Source (Electron) | Rust port |
|--------|-------------------|-----------|
| `gitRoot` | `git-root.cjs` | shell out to `git rev-parse` |
| `git.worktreeList/Add/Remove` | `git-worktree-ops.cjs` | `git worktree` CLI |
| `git.branchSwitch/List` | `git-worktree-ops.cjs` | `git branch` / `git checkout` |
| `git.repoStatus` | `git-review-ops.cjs` | `git status --porcelain` |
| `git.fileDiff` | `git-review-ops.cjs` | `git diff` |
| `git.scanRepos` | `git-repo-scan.cjs` | recursive `.git` discovery |
| `git.review.*` (list, diff, stage, unstage, revert, revParse, commit, commitContext, push, shipInfo, createPr) | `git-review-ops.cjs` | shell out to `git` CLI |

**Preview:**

| Method | Rust implementation |
|--------|---------------------|
| `normalizePreviewTarget` | path normalization |
| `watchPreviewFile` | `notify` crate (file watcher) |
| `stopPreviewFileWatch` | drop watcher |
| `onPreviewFileChanged` | Tauri event from watcher |
| `onClosePreviewRequested` | Tauri event |
| `openPreviewInBrowser` | `open` / `xdg-open` |

**Notifications & clipboard:**

| Method | Rust implementation |
|--------|---------------------|
| `notify` | Tauri notification plugin |
| `onNotificationAction` | Tauri notification action handler |
| `writeClipboard` | Rust clipboard crate |
| `saveImageFromUrl` | `reqwest` + `image` crate |
| `saveImageBuffer` | `std::fs::write` |
| `saveClipboardImage` | clipboard crate + WSL detection |

**Misc:**

| Method | Rust implementation |
|--------|---------------------|
| `api` | HTTP proxy via `reqwest` or renderer fetch with CORS bypass |
| `openExternal` | Tauri shell plugin |
| `fetchLinkTitle` | `reqwest` + HTML parse |
| `requestMicrophoneAccess` | Tauri permission API |
| `settings.getDefaultProjectDir/setDefaultProjectDir/pickDefaultProjectDir` | config file + Tauri dialog |
| `revealLogs` | `open` log dir |
| `getRecentLogs` | read tail of log files |
| `getVersion` | compile-time constant |
| `getRemoteDisplayReason` | env detection (port `bootstrap-platform.cjs`) |
| `onPowerResume` | Tauri power event |
| `setPreviewShortcutActive` | global shortcut registration |

### Tier 3 — Polish (week 3–4)

**Pet overlay (multi-window):**

| Method | Implementation |
|--------|----------------|
| `petOverlay.open` | Tauri: create always-on-top transparent window |
| `petOverlay.close` | close overlay window |
| `petOverlay.setBounds` | window `set_position` / `set_size` |
| `petOverlay.setIgnoreMouse` | window `set_ignore_cursor_events` |
| `petOverlay.setFocusable` | window `set_focusable` |
| `petOverlay.pushState` | IPC to overlay window |
| `petOverlay.onState` | IPC listener in overlay |
| `petOverlay.onControl` | IPC from overlay to main |

**Auto-updater:**

| Method | Source (Electron) | Tauri port |
|--------|-------------------|------------|
| `updates.check` | `update-remote.cjs` + `update-count.cjs` | Tauri updater plugin + custom remote/count logic |
| `updates.apply` | `update-rebuild.cjs` + `update-relaunch.cjs` | Tauri updater `download_and_install` |
| `updates.getBranch/setBranch` | `update-remote.cjs` | config file |
| `updates.onProgress` | `update-marker.cjs` | Tauri updater progress event |

**VSCode marketplace themes:**

| Method | Implementation |
|--------|----------------|
| `themes.fetchMarketplace` | `reqwest` to VS Code marketplace API |
| `themes.searchMarketplace` | same, search endpoint |

**Uninstall:**

| Method | Source | Rust port |
|--------|--------|-----------|
| `uninstall.summary` | `desktop-uninstall.cjs` | scan paths, return summary |
| `uninstall.run` | `desktop-uninstall.cjs` | execute cleanup scripts |

**Bootstrap / installer:**

| Method | Source | Rust port |
|--------|--------|-----------|
| `onBootProgress` | `bootstrap-runner.cjs` | Tauri event |
| `getBootstrapState` | `bootstrap-runner.cjs` | state struct |
| `resetBootstrap` | `bootstrap-runner.cjs` | reset state |
| `repairBootstrap` | `bootstrap-runner.cjs` | re-run install |
| `cancelBootstrap` | `bootstrap-runner.cjs` | cancel + cleanup |
| `onBootstrapEvent` | `bootstrap-runner.cjs` | Tauri event |

**Window chrome theming:**

| Method | Implementation |
|--------|----------------|
| `setTitleBarTheme` | Tauri window `set_theme` + title bar overlay |
| `setNativeTheme` | Tauri window `set_theme` |
| `setTranslucency` | Tauri window `set_transparency` (macOS) |

**Session windows:**

| Method | Source | Rust port |
|--------|--------|-----------|
| `openSessionWindow` | `session-windows.cjs` | Tauri multi-window |
| `openNewSessionWindow` | `session-windows.cjs` | Tauri multi-window |
| `onFocusSession` | `session-windows.cjs` | Tauri window focus event |
| `onWindowStateChanged` | `window-state.cjs` | Tauri window state events |

### Verify

- [ ] Full feature parity checklist: every method in `electron/preload.cjs`
      has a Tauri equivalent that works.
- [ ] Exercise each method from the running desktop app; no
      `not_implemented` errors.
- [ ] Pet overlay opens, renders, drags, returns control to main window.
- [ ] Auto-updater checks, downloads, applies, relaunches.
- [ ] Git review flow (stage → commit → push → create PR) works.
- [ ] File preview watches and updates on change.

---

## 6.4 — Reskin desktop UI

**Goal:** Desktop renderer uses the shared design system. New shell, navbar,
themes.

**Duration:** 2 weeks
**Depends on:** 6.1 (design system)
**Can parallel with:** 6.5

### Tasks

1. Replace `apps/desktop-tauri/src/components/ui/` with imports from
   `@office-agent/design-system`.
2. Redesign `apps/desktop-tauri/src/app/shell/` — new top bar + sidebar
   matching the Open Design reference.
3. Minimal nav groups: collapse agent settings into one collapsible group.
4. Wire 8-theme switcher into the desktop theme menu.
5. Reskin chat chrome around `@assistant-ui/react` (keep transcript / composer
   behavior).
6. Reskin terminal sidebar, settings panels, pet overlay, command palette.
7. Replace `apps/desktop/src/themes/` consumption with shared theme provider.

### Files (modified)

- `apps/desktop-tauri/src/app/shell/*.tsx`
- `apps/desktop-tauri/src/components/**/*.tsx` (swap to design-system imports)
- `apps/desktop-tauri/src/app/desktop-controller.tsx`
- `apps/desktop-tauri/src/themes/` (delegate to shared provider)

### Verify

- [ ] `npm run build` in `apps/desktop-tauri/` passes.
- [ ] All 8 themes switch live.
- [ ] Chat, terminal, settings, pet overlay render in new skin.
- [ ] No console errors.

---

## 6.5 — Reskin web dashboard

**Goal:** `agent-core/web/` uses the shared design system. New shell, navbar,
all pages reskinned.

**Duration:** 2–3 weeks
**Depends on:** 6.1 (design system)
**Can parallel with:** 6.4

### Tasks

1. Redesign `web/src/App.tsx` shell:
   - Single top bar: app name, Marketing | HR switcher, global search,
     notifications, theme toggle.
   - Left sidebar: domain-specific links + collapsible "Agent" group at
     bottom.
   - Remove / redesign `SidebarFooter`, `SidebarStatusStrip`,
     `ProfileSwitcher` to match the reference's bottom-avatar pattern.
2. Replace `@nous-research/ui` imports with `@office-agent/design-system`
   across all pages.
3. Reskin Marketing pages (8): Pipeline, Script, Review, Edit, Publish,
   Comments, Analytics, Catalog.
4. Reskin HR pages (7): Jobs, Pipeline, Candidates, Schedule, Comms,
   Analytics, Hub.
5. Reskin agent pages: Chat, Sessions, Files, Analytics, Models, Logs, Cron,
   Skills, Plugins, MCP, Channels, Webhooks, Pairing, Profiles, Config, Env,
   System, Docs.
6. Wire 8-theme switcher into `web/src/components/ThemeSwitcher.tsx`.
7. Keep persistent chat host behavior (TUI survives tab switches).

### Files (modified)

- `web/src/App.tsx`
- `web/src/components/ThemeSwitcher.tsx`
- `web/src/components/SidebarFooter.tsx`
- `web/src/components/SidebarStatusStrip.tsx`
- `web/src/pages/*.tsx` (all existing pages)
- `web/src/pages/marketing/*.tsx`
- `web/src/pages/hr/*.tsx`
- `web/src/themes/` (delegate to shared provider)

### Verify

- [ ] `npm run build` passes.
- [ ] `npm run typecheck` passes.
- [ ] `npx eslint src/pages/marketing src/pages/hr` passes.
- [ ] All 8 themes switch live.
- [ ] Domain switcher updates nav.
- [ ] Existing routes still reachable.
- [ ] No console errors.

---

## 6.6 — Backend hardening

**Goal:** Move marketing / hr API endpoints from JSON-file stubs to real
MCP-backed behavior.

**Duration:** 1 week
**Depends on:** 6.1 (can run anytime after)
**Can parallel with:** 6.4, 6.5

### Tasks

1. Wire `marketing_api.py` endpoints to MCP tools per
   [`phase-4-ui.md`](./phase-4-ui.md):

   | Endpoint | MCP tool |
   | -------- | -------- |
   | `script/generate` | agent LLM + `mcp-catalog` |
   | `hooks/suggest` | `mcp-ledger.get_hooks_leaderboard` |
   | `edit/render` | `mcp-video-edit.*` chain |
   | `edit/ingest` | `mcp-video-edit.cut_by_shotlist` |
   | `publish/now` | `mcp-social-*.upload` + `mcp-ledger.record_post` |
   | `comments/list` | `mcp-social-*.list_comments` |
   | `comments/reply` | `mcp-social-*.reply` (gated by reply policy) |
   | `analytics/overview` | `mcp-ledger.query_what_worked` |
   | `analytics/drilldown` | `mcp-ledger.query_what_worked` |
   | `catalog` CRUD | `mcp-catalog.*` |

2. Wire `hr_api.py` endpoints:

   | Endpoint | MCP tool |
   | -------- | -------- |
   | `candidates` list / get | `mcp-hr-data` |
   | `compare` | `mcp-cv-screen.compare_candidates` |
   | `screen` | `mcp-cv-screen.score_cv_against_jd` |
   | `schedule/slots` | `mcp-schedule.list_slots` |
   | `schedule/book` | `mcp-schedule.book_slot` |
   | `comms/send` | `messages_send` + log to `comms_log` |
   | `comms/log` | reads `comms_log` via `mcp-hr-data` |
   | `analytics/overview` | aggregates from `mcp-hr-data` |

3. Graceful fallbacks when MCP servers absent (return empty / stub data, log
   warning).

### Files (modified)

- `agent-core/hermes_cli/marketing_api.py`
- `agent-core/hermes_cli/hr_api.py`

### Verify

- [ ] `curl localhost:8000/api/marketing/catalog` → real product data.
- [ ] `curl localhost:8000/api/hr/jobs` → real job data.
- [ ] Pages render with real data when MCP servers configured.
- [ ] Pages render with fallback data when MCP servers absent.

---

## 6.7 — Packaging & cleanup

**Goal:** Ship Tauri builds. Remove Electron. Update CI / docs.

**Duration:** 1 week
**Depends on:** 6.3, 6.4, 6.5

### Tasks

1. Configure Tauri bundler in `apps/desktop-tauri/tauri.conf.json`:
   - `.dmg` + `.zip` (macOS, with notarization)
   - `.msi` + `.nsis` (Windows, with code signing)
   - `.appimage` + `.deb` + `.rpm` (Linux)
2. Auto-updater: configure Tauri updater with signing keys.
3. CI workflows: replace `electron-builder` jobs with `cargo tauri build`
   jobs.
4. Remove:
   - `apps/desktop/electron/`
   - `electron-builder` config in `apps/desktop/package.json`
   - `scripts/stage-native-deps.cjs`
   - `scripts/patch-electron-builder-mac-binary.cjs`
   - Native dep staging in `extraResources`
5. Rename `apps/desktop-tauri/` → `apps/desktop/` (or keep as-is).
6. Update `website/` docs, README, install instructions.
7. Update `agent-core/AGENTS.md` desktop section to reflect Tauri
   architecture.

### Files (removed)

- `apps/desktop/electron/**`
- `apps/desktop/scripts/*electron*`

### Files (modified)

- `apps/desktop-tauri/tauri.conf.json`
- `.github/workflows/*.yml` (CI)
- `agent-core/AGENTS.md`
- `website/docs/**`

### Verify

- [ ] `cargo tauri build` produces installers on all 3 OSes.
- [ ] Installed app launches, backend spawns, all features work.
- [ ] Auto-updater checks and applies a test update.
- [ ] `npm run build` in `web/` still passes.
- [ ] No references to `electron` remain in the codebase.

---

## Phase 6 exit criteria

- [ ] `cargo tauri build` produces signed installers for macOS, Windows,
      Linux.
- [ ] Desktop app launches, spawns backend, chat connects, terminal works,
      all native features functional.
- [ ] No `not_implemented` errors from `window.hermesDesktop`.
- [ ] Web dashboard fully reskinned, `npm run build` clean.
- [ ] Both apps share one design system, 8 themes switch live.
- [ ] Marketing / HR API endpoints backed by real MCP tools (with fallbacks).
- [ ] No Electron code or config remains in the repository.
- [ ] Auto-updater works end-to-end.
- [ ] CI builds and packages the Tauri app on all 3 OSes.

---

## Risk register

| Risk | Mitigation |
|------|------------|
| Rust `portable-pty` doesn't work on WSL | Phase 6.0 spike gates this; fallback to Node sidecar |
| Tauri multi-window pet overlay is buggy | Tier 3 — defer to v1.1 if blocking release |
| Tauri updater doesn't match Electron's branch logic | Custom Rust updater logic, shell out to `git` |
| Reskin breaks existing page behavior | Page-by-page verification, keep old components until verified |
| MCP tools not available for backend hardening | Graceful fallbacks already designed in 6.6 |
| `@assistant-ui/react` version conflicts with new design system | Pin version, test in isolation first |
