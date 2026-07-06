# Hermes PTY Risk Spike

This is the Phase 6.0 risk spike from the Tauri migration plan. It proves that a
Rust-backed PTY can stream shell output into an xterm.js terminal rendered inside
a Tauri webview.

## Status

- [x] PTY backend commands implemented (`terminal_start`, `terminal_write`,
      `terminal_resize`, `terminal_dispose`).
- [x] xterm.js frontend wired to Tauri invoke + events.
- [x] Standalone `portable-pty` test passes on Linux/WSL (read, write, resize,
      exit).
- [ ] Full Tauri app build **blocked on missing Linux GUI dev packages** in this
      environment (see below).

## Files

| File | Purpose |
|------|---------|
| `Cargo.toml` | Tauri + `portable-pty` dependencies |
| `tauri.conf.json` | Tauri v2 app configuration |
| `src/main.rs` | Entry point, registers terminal commands |
| `src/terminal.rs` | PTY session manager + Tauri commands |
| `src/main.ts` | xterm.js frontend |
| `index.html` | Spike UI shell |
| `capabilities/pty-spike.json` | Tauri v2 capability grant |
| `pty-test/` | Standalone `portable-pty` smoke test (no Tauri GUI deps) |

## Running the standalone PTY test

The standalone test does not require webview/GUI system libraries and verifies
that `portable-pty` works on the current OS:

```bash
cd agent-core/apps/desktop-tauri-spike/pty-test
cargo run
```

Expected output ends with:

```
All PTY tests passed on linux
```

## Running the full Tauri spike

```bash
cd agent-core/apps/desktop-tauri-spike
npm install
cargo tauri dev
```

### Linux / WSL prerequisites

If `cargo tauri dev` fails with `pkg-config` or `glib-sys` errors, install the
system development libraries:

```bash
sudo apt-get update
sudo apt-get install -y \
  pkg-config \
  libwebkit2gtk-4.1-dev \
  build-essential \
  curl wget file \
  libxdo3 libssl-dev \
  libgtk-3-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev
```

## Gate decision

- **Pass:** `cargo tauri dev` launches, xterm renders a shell prompt, typing
  `ls` shows output, resizing updates the PTY, closing disposes the process.
- **Fail on WSL only:** fallback to a Node.js sidecar running `node-pty`
  communicating over stdio. The desktop bridge keeps the same shape.
