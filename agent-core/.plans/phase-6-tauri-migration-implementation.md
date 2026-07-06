# Phase 6 — Tauri Migration & Reskin: Implementation Plan

## 1. Executive summary

This plan turns [`.docs/phase-6-tauri-migration.md`](../.docs/phase-6-tauri-migration.md) into an executable build sequence. The goal is:

1. Replace the Electron desktop shell with Tauri.
2. Keep the existing React renderer unchanged by re-implementing `window.hermesDesktop` as a Tauri-backed bridge.
3. Reskin both `agent-core/web/` and the new desktop app with a shared Open Design system package.
4. Harden Marketing / HR APIs to call real MCP servers with graceful fallbacks.

**Duration:** 10–12 weeks (critical path 7 weeks).  
**Team shape:** 1 Rust engineer, 1–2 React engineers, 1 backend engineer (can parallel).

---

## 2. What is in scope

- `agent-core/apps/desktop-tauri/` — new Tauri app (kept separate until 6.7).
- `agent-core/apps/desktop-tauri-spike/` — throwaway 6.0 PTY prototype.
- `agent-core/packages/design-system/` — shared React + Tailwind v4 package.
- `agent-core/web/` — reskin with shared design system.
- `agent-core/hermes_cli/marketing_api.py` + `hr_api.py` — MCP-backed endpoints.
- CI / packaging / docs updates for Tauri.
- Removal of Electron code and configuration.

## 3. What is out of scope (for this phase)

- Mobile apps.
- Rewriting the backend runtime in Rust.
- New agent capabilities beyond the existing Marketing / HR surfaces.

---

## 4. Key architectural decisions

| Decision | Rationale |
|----------|-----------|
| **Bridge-preserving migration** | The renderer never touches Electron directly; it uses `window.hermesDesktop`. Re-implementing that object with Tauri commands keeps the React code almost untouched. |
| **Separate `desktop-tauri` folder until 6.7** | Lets the Electron app keep running for users / QA while the Tauri version matures. Reduces branch churn. |
| **`packages/design-system` as a workspace package** | Both `web/` and `desktop-tauri/src/` import the same primitives, tokens, and provider. |
| **Tailwind v4 CSS-variable-based tokens** | Matches the Open Design reference in `.docs/oa-index.html` and works with the existing Tailwind v4 setup in both apps. |
| **Rust PTY via `portable-pty`** | Required to remove the Node.js sidecar. Phase 6.0 gates this. |
| **Backend still Python** | Tauri only replaces the desktop shell, not the Hermes Python backend. Tauri spawns `hermes serve` and waits for `HERMES_DASHBOARD_READY`. |

---

## 5. Sub-phase execution plan

### 6.0 — PTY risk spike (1 week)

**Goal:** Prove Rust `portable-pty` can stream a shell into xterm.js inside a Tauri webview.

**Deliverables:**
- `agent-core/apps/desktop-tauri-spike/` (throwaway).
- `Cargo.toml`, `tauri.conf.json`, `src/main.rs`, `src/terminal.rs`.
- Minimal HTML/JS frontend with xterm.js.
- Standalone `portable-pty` smoke test (`pty-test/`) that requires no GUI libraries.
- Verification script that runs `ls`, resizes, and closes the PTY.

**Validation performed:**
- `portable-pty` standalone test passed on Linux/WSL: PTY start, write, read,
  resize (`stty size` reports the new dimensions), and clean exit.
- Tauri full-app build is blocked in this environment only by missing Linux
  GUI development packages (`pkg-config`, `libwebkit2gtk-4.1-dev`, etc.), not
  by `portable-pty`.

**Gate:**
- Pass on macOS, Linux, Windows/WSL → proceed.
- Fail on WSL → fallback to Node.js sidecar with `node-pty` over stdio; still
  proceed, but document the extra dependency.

### 6.1 — Shared design system package (2 weeks)

**Goal:** One package consumed by both `web/` and `desktop-tauri/`.

**Workspaces change:**
- Add `packages/*` to `agent-core/package.json` workspaces.
- Create `agent-core/packages/design-system/`.

**Package contents:**
- `package.json` — name `@office-agent/design-system`, exports `./themes`, `./components`, `./tailwind-preset`.
- `src/themes/tokens.css` — 8 palettes × light/dark from `.docs/oa-index.html`.
- `src/themes/provider.tsx` — `ThemeProvider` + `useTheme`; sets `data-theme` / `data-mode` and persists to localStorage.
- `src/themes/presets.ts` — theme catalog matching the 8 themes.
- `src/components/*.tsx` — Button, Card, Input, Textarea, Select, Badge, Avatar, Spinner, Skeleton, Modal, Table, NavLink, NavGroup, TopBar, Sidebar, Toast.
- `tailwind.preset.ts` — Tailwind v4 preset exporting CSS-variable color tokens.
- `demo/index.html` — Storybook-style visual verification page.

**Verification:**
- `npm run build` in package passes.
- Demo page renders all 8 themes with live switching.
- Both `web/` and `apps/desktop-tauri/` can import `{ Button }`.
- Light/dark toggle works without reload.

### 6.2 — Tauri shell + `hermesDesktop` bridge (2 weeks)

**Goal:** A Tauri app launches, spawns the backend, and exposes `window.hermesDesktop` matching `electron/preload.cjs`.

**Structure:**
```
agent-core/apps/desktop-tauri/
  Cargo.toml
  tauri.conf.json
  src/
    main.rs          # entry, window mgmt, deep-link, tray
    backend.rs       # spawn hermes serve, env, ready probe, token exchange
    connection.rs    # connection config, profile, gateway URL
    bridge.rs        # Tauri commands + event dispatch
    terminal.rs      # PTY (ported from 6.0 spike)
    file_system.rs   # std::fs ops
    git.rs           # git CLI wrappers
    preview.rs       # file watcher, browser open
    notifications.rs # Tauri notification plugin wrapper
    clipboard.rs     # clipboard + image helpers
    settings.rs      # default project dir, dialogs
    updater.rs       # Tauri updater + branch logic
    session_windows.rs # multi-window helpers
    pet_overlay.rs   # transparent always-on-top window
  src/renderer/      # copied unmodified from apps/desktop/src/
  src/preload.ts   # injects window.hermesDesktop backed by @tauri-apps/api
```

**Critical path for 6.2 boot:**
1. `getConnection` / `revalidateConnection` / `touchBackend`
2. `getGatewayWsUrl`
3. `getConnectionConfig` / `saveConnectionConfig` / `applyConnectionConfig`
4. `profile.get` / `profile.set`
5. `onDeepLink` / `signalDeepLinkReady`
6. `terminal.*`
7. `onBackendExit`

All other methods return `not_implemented` so the renderer fails gracefully during 6.3.

### 6.3 — Port all native features (3–4 weeks)

Tiered implementation of every `window.hermesDesktop` method.

**Tier 1 (already done in 6.2, verify & harden):**
Connection, profile, terminal, deep links, backend lifecycle.

**Tier 2 (core features):**

| Area | Methods | Rust approach |
|------|---------|---------------|
| File system | `readFileDataUrl`, `readFileText`, `readDir`, `writeTextFile`, `renamePath`, `trashPath`, `revealPath`, `sanitizeWorkspaceCwd` | `std::fs` + `trash` crate + `opener` / platform commands |
| Git | `gitRoot`, `worktreeList/Add/Remove`, `branchSwitch/List`, `repoStatus`, `fileDiff`, `scanRepos`, `review.*` | Shell out to `git` CLI; reuse exact argument shapes from `electron/git-*.cjs` |
| Preview | `normalizePreviewTarget`, `watchPreviewFile`, `stopPreviewFileWatch`, `onPreviewFileChanged`, `onClosePreviewRequested`, `openPreviewInBrowser` | `notify` crate for watcher; `opener` for browser |
| Notifications & clipboard | `notify`, `onNotificationAction`, `writeClipboard`, `saveImageFromUrl`, `saveImageBuffer`, `saveClipboardImage` | Tauri notification plugin; `arboard` clipboard crate; `reqwest` + `image` |
| Misc | `api`, `openExternal`, `fetchLinkTitle`, `requestMicrophoneAccess`, `settings.*`, `revealLogs`, `getRecentLogs`, `getVersion`, `getRemoteDisplayReason`, `onPowerResume`, `setPreviewShortcutActive` | `reqwest` HTTP proxy; Tauri shell plugin; platform permission APIs; config file |

**Tier 3 (polish, can slip to post-release):**
- Pet overlay multi-window.
- Auto-updater branch logic.
- VSCode marketplace themes.
- Uninstall / bootstrap runner.
- Window chrome theming.
- Session windows.

### 6.4 — Reskin desktop UI (2 weeks)

- Replace `apps/desktop-tauri/src/components/ui/` with imports from `@office-agent/design-system`.
- Redesign shell: top bar, sidebar, nav groups, bottom avatar.
- Wire 8-theme switcher into desktop theme menu.
- Reskin chat chrome around `@assistant-ui/react` (keep behavior).
- Reskin terminal sidebar, settings, pet overlay, command palette.

### 6.5 — Reskin web dashboard (2–3 weeks)

- Replace `@nous-research/ui` imports with `@office-agent/design-system`.
- Redesign `web/src/App.tsx` shell per Open Design reference.
- Reskin all Marketing / HR / Agent pages.
- Keep persistent chat host behavior.

### 6.6 — Backend hardening (1 week)

- Wire `marketing_api.py` and `hr_api.py` endpoints to MCP tools per the doc.
- Add graceful fallbacks when MCP servers are absent.

### 6.7 — Packaging & cleanup (1 week)

- Configure Tauri bundler: `.dmg`/`.zip` (macOS), `.msi`/`.nsis` (Windows), `.appimage`/`.deb`/`.rpm` (Linux).
- Configure Tauri updater with signing keys.
- Replace `electron-builder` CI jobs with `cargo tauri build`.
- Remove Electron code and scripts.
- Rename `apps/desktop-tauri/` → `apps/desktop/` (optional; recommended).
- Update README, install docs, `AGENTS.md`.

---

## 6. File structure after migration

```
agent-core/
  packages/
    design-system/
      package.json
      src/
        index.ts
        themes/
          tokens.css
          presets.ts
          provider.tsx
          types.ts
        components/
          Button.tsx
          Card.tsx
          ...
      tailwind.preset.ts
      demo/
        index.html
  apps/
    desktop-tauri/          # (renamed to desktop in 6.7)
      Cargo.toml
      tauri.conf.json
      src/
        main.rs
        backend.rs
        connection.rs
        bridge.rs
        terminal.rs
        file_system.rs
        git.rs
        preview.rs
        notifications.rs
        clipboard.rs
        settings.rs
        updater.rs
        session_windows.rs
        pet_overlay.rs
        preload.ts
      src/renderer/          # copied from old apps/desktop/src/
        App.tsx
        hermes.ts
        components/
        app/
        themes/             # delegates to shared provider
    desktop/                 # removed in 6.7
  web/
    src/
      App.tsx               # reskinned
      themes/               # delegates to shared provider
      components/           # use @office-agent/design-system
      pages/                # reskinned
  hermes_cli/
    marketing_api.py        # MCP-backed
    hr_api.py               # MCP-backed
```

---

## 7. Verification strategy

| Stage | Verification |
|-------|--------------|
| 6.0 | `ls` in xterm renders; resize sends SIGWINCH; close disposes PTY; passes on 3 OSes. |
| 6.1 | Package builds; demo page shows all themes; both apps import components. |
| 6.2 | `cargo tauri dev` launches; backend spawns; chat connects; terminal works; deep links resolve. |
| 6.3 | Headless bridge test: every `window.hermesDesktop` method returns without `not_implemented`. Manual feature parity checklist. |
| 6.4 | `npm run build` passes; all 8 themes switch; chat/terminal/settings render in new skin. |
| 6.5 | `npm run build` + `npm run typecheck` pass; all themes switch; routes reachable; no console errors. |
| 6.6 | `curl` endpoints return real data with MCPs, fallback data without. |
| 6.7 | `cargo tauri build` produces installers on all 3 OSes; auto-updater applies test update; no `electron` references remain. |

---

## 8. Risks and mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| `portable-pty` fails on WSL | Medium | High | 6.0 spike gates it; standalone test proves it works on WSL. If full Tauri build fails, fallback to Node sidecar. |
| Missing Linux GUI dev packages blocks Tauri build | High (in fresh WSL) | High | Install `libwebkit2gtk-4.1-dev`, `pkg-config`, `libgtk-3-dev`, `libayatana-appindicator3-dev`, `librsvg2-dev`, `libssl-dev` (see spike README). |
| Tauri multi-window pet overlay is buggy | Medium | Medium | Tier 3; can slip to v1.1. |
| Tauri updater branch logic differs from Electron | Medium | High | Custom Rust updater logic wrapping Tauri plugin. |
| Reskin breaks existing page behavior | Medium | Medium | Page-by-page verification; keep old imports behind a temporary flag until verified. |
| `@assistant-ui/react` conflicts with design system | Low | Medium | Pin version; test in isolation first. |
| Signing keys / Apple certs not ready by 6.7 | Medium | High | Start procurement in week 2; use unsigned test builds meanwhile. |

---

## 9. Immediate next steps (after plan approval)

1. Add `packages/*` to `agent-core/package.json` workspaces.
2. Scaffold `agent-core/apps/desktop-tauri-spike/`.
3. Implement the 6.0 PTY spike and run the gate verification.
4. If the gate passes, begin 6.1 + 6.2 in parallel.

---

## 10. Open questions to resolve during implementation

1. Should `apps/desktop-tauri/` be renamed to `apps/desktop/` immediately, or only after Electron removal in 6.7?  
   **Recommendation:** keep `desktop-tauri` until 6.7 to avoid disrupting the Electron build.
2. Do we keep `@nous-research/ui` as a transitive dependency for complex components (e.g., charts) or remove it entirely?  
   **Recommendation:** remove direct imports and wrap/replace as needed; keep only if a component is too expensive to re-implement.
3. Should the design-system package be published to npm, or consumed via workspace symlink?  
   **Recommendation:** workspace symlink for now; publish only if other repos need it.
4. What is the exact backend binary discovery strategy on Windows (`.exe` name, PATH fallback, bundled resource)?  
   **Recommendation:** port `electron/backend-command.cjs` logic to Rust during 6.2.
