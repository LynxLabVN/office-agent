# Multi-App Video-Edit MCP — Insight (deferred)

Status: deferred. Build automation first, revisit later.
Date noted: 2026-07-23

## Context

`mcp-video-edit` currently = thin ffmpeg/ffprobe/whisper CLI wrapper (stdio MCP).
Read input, write new output to `EDIT_WORKSPACE` (`~/.hermes/work`). Input untouched.

Current tools: `cut_by_shotlist`, `concat`, `burn_captions`, `overlay_text`,
`add_music`, `encode_916`, `extract_audio`, `health`.

- cut_by_shotlist = lossless stream copy (`-c copy`), no re-encode, no quality loss.
- Others = re-encode (filters need decode→encode), generation loss, slower.

## Question

Could mcp-video-edit become an MCP agent tool that drives common GUI video editors
(CapCut, Premiere, etc.) — since those apps have more complex-work support functions?

## Verdict

Possible for some, fragile for most. ffmpeg stays the stable core.

### Automation surface per app

| App | Scripting/API | MCP feasibility |
|-----|---------------|-----------------|
| DaVinci Resolve | Official Python + Lua API (Studio). Timeline, nodes, render queue, color, Fusion | Strong — best of consumer/pro |
| Premiere Pro | ExtendScript (JSX) + CEP/UXP panels | Medium — fragile, version-dependent |
| Final Cut Pro | No official API. fcpxml interchange only. AppleScript nearly nil | Weak — XML roundtrip only |
| Vegas Pro | C# / .NET scripting API (VegasScript) | Medium — less brittle than Adobe |
| Avid Media Composer | Limited scripting, AMA SDK | Weak |
| Blender (VSE) | Full bpy Python API, headless `blender -b -P` | Strong |
| OpenShot | libopenshot Python API (ffmpeg+MLT) | Medium |
| Kdenlive / Shotcut | MLT `melt` CLI + MLT XML | Strong — headless, stable |
| Olive | Python scripting (older) | Weak — small project |
| CapCut | None public | None |

### Tiers

Tier 1 — clean MCP target (official stable API, wrappable):
- DaVinci Resolve (Python/Lua) — richest.
- Blender VSE (bpy).
- MLT-based (Kdenlive/Shotcut via `melt`) — closest to ffmpeg philosophy, headless.

Tier 2 — possible but fragile:
- Premiere (JSX/CEP), Vegas (C#).

Tier 3 — export/import XML roundtrip only, no live control:
- Final Cut Pro (fcpxml).

Dead end:
- CapCut, Avid practical, Olive small.

### Caveats

- Premiere/CEP: Adobe API unstable, version-breaking, GUI must be running,
  selectors/effect IDs undocumented. Headless impossible.
- CapCut: no public scripting/plugin API, `.draft` format undocumented and
  changes per version — reverse-engineering breaks on every update. Don't pursue officially.
- Final Cut Pro: live control not possible, only fcpxml roundtrip.

## Proposed architecture (when revisited)

Multi-app video-edit MCP exposes common tool set (`cut`, `overlay`, `render`,
`color_grade`) and dispatches per-app backend:

- ffmpeg = default headless engine (already built).
- DaVinci = wrap `DaVinciResolveScript.py` API.
- Blender = wrap `bpy` via `blender -b -P script.py`.
- Kdenlive/Shotcut = generate MLT XML, run `melt`.

Value-add layer over ffmpeg: color grading, node compositing, multi-track,
effects — the complex-work functions ffmpeg lacks. DaVinci + Blender + MLT
deliver that. CapCut/Premiere do not add enough over ffmpeg to justify fragility.

## Next (when ready)

1. Decide scope: pick Tier-1 target(s) — DaVinci + MLT likely highest ROI.
2. Spec common tool interface + per-app backend dispatch.
3. Prototype DaVinci backend via `DaVinciResolveScript.py`.
4. Prototype MLT backend (`melt` + XML gen).