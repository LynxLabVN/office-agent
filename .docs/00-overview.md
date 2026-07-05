# 00 — Overview: Workspace Structure & Conventions

Read this before starting any phase. Every phase file assumes the structure
and conventions below.

## Workspace target structure (final state)

```
office-agent/
├── .docs/                            ← this build manual (split per phase)
│   ├── README.md                     ← index + dependency graph + DoD
│   ├── 00-overview.md                ← this file
│   ├── phase-0-bootstrap.md
│   ├── phase-1-marketing-mcp-p0.md
│   ├── phase-1b-hr-mcp-p0.md
│   ├── phase-2-skills-cli-cron.md
│   ├── phase-3-mcp-p1-oauth.md
│   ├── phase-4-ui.md
│   └── phase-5-hardening.md
├── PLAN.md                           ← architecture + risks + integration rules
├── LEADTIMES.md                      ← tracker for app-review waits (Phase 0)
├── agent-core/                       ← LynxLabVN/agent-core (Hermes base)
│   ├── run_agent.py                  ← NO modification (prompt caching sacred)
│   ├── hermes_cli/
│   │   ├── web_server.py             ← extend (additive API endpoints)
│   │   └── web_dist/                 ← built React app lands here
│   ├── skills/
│   │   ├── social-media/             ← existing, extend
│   │   ├── marketing/marketing-pipeline/SKILL.md  ← new
│   │   ├── hr/recruitment/SKILL.md   ← new
│   │   ├── reply-policy/             ← shared module (Phase 3)
│   │   ├── quota-budgeter/           ← shared module (Phase 5)
│   │   └── audit/                    ← shared module (Phase 5)
│   ├── cron/                         ← existing, register new jobs
│   └── web/
│       └── src/
│           ├── pages/                ← extend with marketing + HR pages
│           │   ├── marketing/        ← 8 new pages
│           │   └── hr/               ← 6 new pages
│           └── lib/api.ts            ← extend API client
└── optional-mcps/
    ├── Cargo.toml                    ← Rust workspace root
    ├── rust-toolchain.toml           ← pin stable
    ├── SMOKE.md                      ← integration smoke transcripts (Phase 1)
    ├── mcp-catalog/                  ← Rust crate
    ├── mcp-video-edit/               ← Rust crate
    ├── mcp-ledger/                   ← Rust crate
    ├── mcp-social-youtube/           ← Rust crate
    ├── mcp-social-meta/              ← Rust crate
    ├── mcp-social-tiktok/            ← Rust crate
    ├── mcp-trend-research/           ← Rust crate
    ├── mcp-hr-data/                  ← Rust crate
    ├── mcp-cv-screen/                ← Python package (ONLY Python MCP)
    ├── mcp-schedule/                 ← Rust crate
    └── mcp-zalo-oa/                  ← Rust crate
```

## Conventions (apply to every phase)

| Item | Choice | Note |
| ---- | ------ | ---- |
| Rust MCP SDK | `rmcp` (official MCP Rust SDK) | stdio transport |
| Rust toolchain | stable, edition 2021 | pin via `rust-toolchain.toml` |
| Async runtime | `tokio` multi-thread | |
| HTTP client | `reqwest` with rustls | no openssl dep |
| SQLite | `rusqlite` bundled | single static binary |
| Serialization | `serde` + `serde_json` | |
| Errors | `anyhow` (apps) / `thiserror` (libs) | |
| Python MCP | FastMCP + PyMuPDF + Pillow + faster-whisper | only `mcp-cv-screen` |
| MCP registration | `optional-mcps/<name>/manifest.toml` | binary path, env, scopes |
| Build verify | `cargo build --workspace --release` | |
| Test verify | `cargo test --workspace` | |
| Integration verify | `hermes mcp list` then `hermes mcp call <name> <tool> <json>` | |

**Rule:** every Rust MCP crate has the same skeleton:
`src/main.rs` (entry, rmcp server start) + `src/tools.rs` (tool handlers) +
`src/db.rs` or `src/client.rs` (state) + `Cargo.toml`.

**Rule:** every MCP exposes a `health` tool returning `{"ok": true, "name":
"<name>"}` for smoke-testing.

**Rule:** no modification to `agent-core/run_agent.py` or `toolsets.py` —
the core agent loop and prompt caching are sacred. All extension is additive
(MCPs + skills + UI pages + CLI commands + cron jobs).

**Rule:** third-party product integrations ship as standalone MCP crates under
`optional-mcps/`, registered via `manifest.toml`, never in-tree to agent-core.

**Rule:** external integrations follow the 4-tier order from PLAN.md —
official API → own controlled surface → human-in-loop manual → browser
automation (last resort, never load-bearing).

## Workspace `Cargo.toml` (reference — written in Phase 0)

```toml
[workspace]
resolver = "2"
members = [
    "mcp-catalog",
    "mcp-video-edit",
    "mcp-ledger",
    "mcp-social-youtube",
    "mcp-social-meta",
    "mcp-social-tiktok",
    "mcp-trend-research",
    "mcp-hr-data",
    "mcp-schedule",
    "mcp-zalo-oa",
]

[workspace.package]
version = "0.1.0"
edition = "2021"
license = "MIT"

[workspace.dependencies]
rmcp = { version = "0.1", features = ["server"] }
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
rusqlite = { version = "0.31", features = ["bundled"] }
reqwest = { version = "0.12", default-features = false, features = ["rustls-tls", "json", "multipart"] }
anyhow = "1"
thiserror = "1"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```

## MCP inventory (11 total — 10 Rust + 1 Python)

| MCP | Lang | Phase | Priority | Auth |
| --- | ---- | ----- | -------- | ---- |
| `mcp-catalog` | Rust | 1 | P0 | none (SQLite) |
| `mcp-video-edit` | Rust | 1 | P0 | none (FFmpeg) |
| `mcp-ledger` | Rust | 1 | P0 | none (SQLite) |
| `mcp-hr-data` | Rust | 1b | P0 | none (SQLite) |
| `mcp-cv-screen` | Python | 1b | P0 | none (LLM via agent) |
| `mcp-social-youtube` | Rust | 3 | P1 | OAuth2 (Google) |
| `mcp-social-meta` | Rust | 3 | P1 | OAuth2 (Meta Business) |
| `mcp-social-tiktok` | Rust | 3 | P1 | OAuth2 (Content Posting) |
| `mcp-schedule` | Rust | 3 | P1 | Cal.com API key |
| `mcp-zalo-oa` | Rust | 3 | P1 | Zalo OA long-lived token |
| `mcp-trend-research` | Rust | 5 | P2 | TT Research + YT Data |

Plus existing `hermes-zalo-plugin` (Node, installed in Phase 0) for personal
Zalo — not an MCP, used via Hermes gateway `messages_send`.
