# office-agent — Build Docs Index

Step-by-step build manual, split per phase. Each phase file lists exact files,
schemas, signatures, commands, and verification steps. Execute in order unless
a phase is marked **parallelizable**. Do not skip the **Verify** step of any
task — it is the acceptance criterion.

## Read first

- [`00-overview.md`](./00-overview.md) — workspace target structure + conventions (read before any phase)

## Phases

| File | Phase | Depends on | Duration | Parallel with |
| ---- | ----- | ---------- | -------- | ------------- |
| [`phase-0-bootstrap.md`](./phase-0-bootstrap.md) | Phase 0 — Bootstrap & parallel lead-times | nothing | ~2-3 days + weeks waiting | — (foundation) |
| [`phase-1-marketing-mcp-p0.md`](./phase-1-marketing-mcp-p0.md) | Phase 1 — Marketing MCP P0 (Rust) | Phase 0 | ~1 week | Phase 1b |
| [`phase-1b-hr-mcp-p0.md`](./phase-1b-hr-mcp-p0.md) | Phase 1b — HR MCP P0 (Rust + Python) | Phase 0 | ~3-4 days | Phase 1 |
| [`phase-2-skills-cli-cron.md`](./phase-2-skills-cli-cron.md) | Phase 2 — Skills + CLI + cron (both domains) | Phase 1 + 1b | ~5 days | Phase 3 |
| [`phase-3-mcp-p1-oauth.md`](./phase-3-mcp-p1-oauth.md) | Phase 3 — MCP P1 (OAuth-gated) | Phase 0 app reviews + Phase 1 patterns | ~1.5-2 weeks | Phase 2 |
| [`phase-4-ui.md`](./phase-4-ui.md) | Phase 4 — UI redesign (additive) | Phase 1 + 1b + 2 | ~1.5 weeks | Phase 5 |
| [`phase-5-hardening.md`](./phase-5-hardening.md) | Phase 5 — P2 + hardening | Phase 1 + 1b + 3 | ~1 week | Phase 4 |
| [`phase-6-tauri-migration.md`](./phase-6-tauri-migration.md) | Phase 6 — Tauri migration & full reskin | Phase 4 + 5 | ~10–12 weeks | — (final track) |

## Cross-phase dependency graph

```
Phase 0 ──┬─→ Phase 1 (Marketing P0)  ──┐
          ├─→ Phase 1b (HR P0)         ──┤
          │                              ├─→ Phase 2 (Skills)  ──┐
          │                              │                        ├─→ Phase 4 (UI)
          └─→ [app reviews] ─→ Phase 3 (MCP P1) ──────────────────┤
                                                                     │
                                  Phase 5 (Hardening) ←──────────────┘
                                          │
                                          ↓
                                  Phase 6 (Tauri migration)
                                  ├── 6.0 PTY spike (gate)
                                  ├── 6.1 Design system
                                  ├── 6.2 Tauri shell + bridge
                                  ├── 6.3 Native features
                                  ├── 6.4 Desktop reskin
                                  ├── 6.5 Web reskin
                                  ├── 6.6 Backend hardening
                                  └── 6.7 Packaging & cleanup
```

- Phase 1 and 1b run in parallel.
- Phase 2 needs 1 + 1b (skills call MCP tools).
- Phase 3 needs app reviews (Phase 0) + can run parallel with Phase 2.
- Phase 4 needs 1 + 1b + 2 (UI wraps existing MCP tools). Stubs for Phase 3
  tools if not ready.
- Phase 5 needs 1 + 1b + 3. Can run parallel with Phase 4.
- Phase 6 needs 4 + 5. Sub-phases 6.1 / 6.4 / 6.5 can parallel after the 6.0
  gate; 6.6 can run anytime after 6.1.

## Definition of done (whole project)

- [ ] `cargo build --workspace --release` — 10 Rust binaries
- [ ] `pip install -e optional-mcps/mcp-cv-screen` works
- [ ] `hermes mcp list` — 11 MCPs + existing, all `connected`
- [ ] `hermes marketing new MA5` runs full pipeline to manager gate
- [ ] `hermes hr new-job` → post → screen CVs → schedule interview
- [ ] Dashboard: 14 new pages + domain switcher, `npm run build` clean
- [ ] `hermes cron list` — 5 jobs, all executable
- [ ] Audit log covers a full marketing + HR cycle
- [ ] CV files encrypted at rest
- [ ] Reply policy engine gates all auto-replies
- [ ] No modification to `run_agent.py` or `toolsets.py`
- [ ] `cargo tauri build` produces signed installers for macOS, Windows, Linux
- [ ] Desktop app runs on Tauri (no Electron), all native features functional
- [ ] Both web + desktop share one design system, 8 themes switch live
- [ ] No Electron code or config remains in the repository

## Related

- [`PLAN.md`](./PLAN.md) — architecture, MCP inventory, risks, footprint ladder, integration-order rule, bot-detection tier table.
