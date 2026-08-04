#!/usr/bin/env python3
"""DaVinci Resolve backend bridge for mcp-video-edit.

Invoked by the Rust `DavinciEngine` per tool call:
    python3 davinci_bridge.py <command>          # reads JSON args from stdin

Emits exactly one JSON object on stdout:
    {"output_path": "...", "segment_count": N, "engine": "davinci"}  on success
    {"error": "..."}                                                 on failure

Env:
    RESOLVE_PROJECT   project name (default: mcp-e2e)
    EDIT_WORKSPACE    output workspace (default: ~/.hermes/work)
    DAVINCI_SCRIPT_PATH  optional path to DaVinciResolveScript.py parent dir
                        (added to sys.path before import)

Requires DaVinci Resolve to be **running** (scripting API talks to the live app).

API reference: DaVinci Resolve 18/19 Python scripting. Some call shapes
(CreateTimelineFromClips, AddClipsToTimeline clip-info dicts, SetRenderSettings)
are version-sensitive; this bridge assumes Resolve >= 18. Confirm against live
Resolve per DAVINCI_E2E_TEST.md.
"""

from __future__ import annotations

import json
import os
import sys
from pathlib import Path


# ---------------------------------------------------------------------------
# Resolve connection
# ---------------------------------------------------------------------------

def _resolve():
    """Return the live Resolve scripting app, importing modules robustly."""
    script_path = os.environ.get("DAVINCI_SCRIPT_PATH")
    if script_path and script_path not in sys.path:
        sys.path.insert(0, script_path)
    # Resolve's installer also exposes the module via env set by the launcher;
    # if DAVINCI_SCRIPT_PATH is unset, rely on PYTHONPATH/installed modules.
    try:
        import DaVinciResolveScript as dvr  # type: ignore
    except Exception as exc:
        raise RuntimeError(
            "cannot import DaVinciResolveScript — set DAVINCI_SCRIPT_PATH to the "
            "dir containing DaVinciResolveScript.py / the Modules dir "
            f"(got: {exc})"
        ) from exc
    resolve = dvr.scriptapp()
    if resolve is None:
        raise RuntimeError(
            "DaVinciResolveScript.scriptapp() returned None — is Resolve running?"
        )
    return resolve


def _project(resolve, name: str):
    """Load an existing project by name, or create it."""
    pm = resolve.GetProjectManager()
    if pm is None:
        raise RuntimeError("GetProjectManager() returned None")
    project = pm.LoadProject(name)
    if project is None:
        project = pm.CreateProject(name)
    if project is None:
        raise RuntimeError(f"could not load or create project {name!r}")
    return project


def _frame_rate(project) -> float:
    """Project timeline frame rate as float (e.g. 24.0, 30.0)."""
    rate = project.GetSetting("timelineFrameRate")
    try:
        return float(rate)
    except (TypeError, ValueError):
        return 24.0  # safe fallback


def _seconds_to_frames(project, sec: float) -> int:
    return int(round(sec * _frame_rate(project)))


def _workspace() -> Path:
    ws = os.environ.get("EDIT_WORKSPACE") or str(Path.home() / ".hermes" / "work")
    p = Path(ws)
    p.mkdir(parents=True, exist_ok=True)
    return p


def _import_clip(media_pool, root_folder, abs_path: str):
    """Import media if not already in the folder; return the MediaPoolItem."""
    for clip in root_folder.GetClipList():
        if clip.GetFilePath() == abs_path:
            return clip
    imported = media_pool.ImportMedia([abs_path])
    if not imported:
        raise RuntimeError(f"ImportMedia failed for {abs_path}")
    # ImportMedia returns the clips it added.
    return imported[0]


# ---------------------------------------------------------------------------
# Commands
# ---------------------------------------------------------------------------

def cmd_health(args):
    resolve = _resolve()
    return {"ok": True, "engine": "davinci", "name": resolve.GetProductName()}


def cmd_cut(args):
    """Create a timeline from a shot list (per-shot in/out), in order."""
    resolve = _resolve()
    project = _project(resolve, os.environ.get("RESOLVE_PROJECT", "mcp-e2e"))
    mp = project.GetMediaPool()
    root = mp.GetRootFolder()

    input_path = os.path.abspath(args["input"])
    clip = _import_clip(mp, root, input_path)

    shots = args["shots"]
    if not shots:
        raise ValueError("shots cannot be empty")

    clip_infos = []
    for shot in shots:
        start = _seconds_to_frames(project, shot["start_sec"])
        end = _seconds_to_frames(project, shot["end_sec"])
        if end <= start:
            raise ValueError(f"invalid shot range: {start}-{end}")
        clip_infos.append({
            "mediaPoolItem": clip,
            "startFrame": start,
            "endFrame": end,
            "trackIndex": 0,
        })

    timeline_name = f"mcp_cut_{os.getpid()}"
    timeline = mp.CreateTimelineFromClips(timeline_name, [c["mediaPoolItem"] for c in clip_infos])
    if timeline is None:
        # Fallback: append clips with in/out onto a fresh timeline.
        timeline = mp.CreateEmptyTimeline(timeline_name)
        if timeline is None:
            raise RuntimeError("CreateTimelineFromClips/CreateEmptyTimeline returned None")
        ok = mp.AppendClipsToTimeline(clip_infos)
        if not ok:
            raise RuntimeError("AppendClipsToTimeline failed")

    return {
        "output_path": f"timeline:{timeline_name}",
        "segment_count": len(shots),
        "engine": "davinci",
    }


def _current_timeline(project):
    tl = project.GetCurrentTimeline()
    if tl is None:
        count = project.GetTimelineCount()
        if count > 0:
            tl = project.GetTimelineByIndex(count)
    if tl is None:
        raise RuntimeError("no current timeline to operate on")
    return tl


def cmd_overlay(args):
    """Overlay text on the current timeline (Text+ Fusion effect on track 2)."""
    resolve = _resolve()
    project = _project(resolve, os.environ.get("RESOLVE_PROJECT", "mcp-e2e"))
    timeline = _current_timeline(project)

    overlays = args["overlays"]
    for ov in overlays:
        start = _seconds_to_frames(project, ov["start_sec"])
        end = _seconds_to_frames(project, ov["end_sec"])
        text = ov["text"]
        # Add a Fusion Text+ effect. Effect list / generator insertion is
        # version-sensitive; the generic path is AddFusionEffectByIndex on a
        # timeline item, but inserting a generator onto track 2 is not directly
        # exposed in older builds. We attempt the generator approach and fall back
        # to a clip-level Fusion effect on the first video item.
        ok = False
        try:
            ok = timeline.AddGenerator("Text", trackIndex=1, recordFrame=start, duration=end - start)
        except Exception:
            ok = False
        if not ok:
            # Fallback: attach a Fusion effect to the first video clip.
            items = timeline.GetItemListInTrack("video", 1)
            if items:
                try:
                    items[0].AddFusionEffectByIndex("Text", start, end, text)
                    ok = True
                except Exception:
                    ok = False
        if not ok:
            raise RuntimeError(
                "could not add text overlay — Resolve build does not expose a "
                "supported text-insertion call (see DAVINCI_E2E_TEST.md gotchas)"
            )
    return {"output_path": f"timeline:{timeline.GetName()}", "engine": "davinci"}


def cmd_burn_captions(args):
    """Burn captions as Text+ generators on the current timeline."""
    resolve = _resolve()
    project = _project(resolve, os.environ.get("RESOLVE_PROJECT", "mcp-e2e"))
    timeline = _current_timeline(project)

    captions = args.get("captions")
    if captions is None:
        raise RuntimeError("auto-transcribe (whisper) not supported in davinci engine; pass captions")
    for cap in captions:
        start = _seconds_to_frames(project, cap["start_sec"])
        end = _seconds_to_frames(project, cap["end_sec"])
        try:
            ok = timeline.AddGenerator("Text", trackIndex=1, recordFrame=start, duration=end - start)
        except Exception:
            ok = False
        if not ok:
            raise RuntimeError("could not add caption generator (Text generator unsupported)")
    return {"output_path": f"timeline:{timeline.GetName()}", "engine": "davinci"}


def cmd_add_music(args):
    """Add a music track to the current timeline."""
    resolve = _resolve()
    project = _project(resolve, os.environ.get("RESOLVE_PROJECT", "mcp-e2e"))
    timeline = _current_timeline(project)
    mp = project.GetMediaPool()
    root = mp.GetRootFolder()

    music_path = os.path.abspath(args["music"])
    music_clip = _import_clip(mp, root, music_path)

    ok = mp.AppendClipsToTimeline([{
        "mediaPoolItem": music_clip,
        "trackIndex": 0,  # first available audio track after video
        "mediaType": 1,    # audio
    }])
    if not ok:
        raise RuntimeError("AppendClipsToTimeline failed for music clip")
    return {"output_path": f"timeline:{timeline.GetName()}", "engine": "davinci"}


def cmd_render(args):
    """Render the current timeline to a file (encode_916 -> 9:16 vertical)."""
    resolve = _resolve()
    project = _project(resolve, os.environ.get("RESOLVE_PROJECT", "mcp-e2e"))
    timeline = _current_timeline(project)
    project.SetCurrentTimeline(timeline)

    ws = _workspace()
    out_dir = str(ws / "davinci-out")
    Path(out_dir).mkdir(parents=True, exist_ok=True)
    out_name = f"r_{os.getpid()}.mov"

    # Codec: prefer prores (works on Linux free + Studio). On Windows free,
    # H.264 is available — caller may request via args.get("codec").
    codec = args.get("codec", "prores")
    render_settings = {
        "TargetDir": out_dir,
        "OutputFilename": out_name,
        "VideoCodec": codec,
        "AudioCodec": "AAC",
        "FormatWidth": 1080,
        "FormatHeight": 1920,
        "Width": 1080,
        "Height": 1920,
    }
    try:
        project.SetRenderSettings(render_settings)
    except Exception as exc:
        raise RuntimeError(f"SetRenderSettings failed: {exc}")

    job_id = project.AddRenderJob()
    if not job_id:
        raise RuntimeError("AddRenderJob returned empty/None")

    if not project.StartRendering(job_id):
        raise RuntimeError("StartRendering failed")

    # Poll until done (synchronous; keep test clips short).
    import time
    while project.IsRenderingInProgress():
        time.sleep(0.5)

    status = project.GetRenderJobStatus(job_id) or {}
    if status.get("JobStatus", "").lower() not in ("complete", "completed"):
        raise RuntimeError(f"render did not complete: {status}")

    out_path = os.path.join(out_dir, out_name)
    return {"output_path": out_path, "width": 1080, "height": 1920, "engine": "davinci"}


def cmd_concat(args):
    """Concatenate multiple clips into a new timeline (no transition)."""
    resolve = _resolve()
    project = _project(resolve, os.environ.get("RESOLVE_PROJECT", "mcp-e2e"))
    mp = project.GetMediaPool()
    root = mp.GetRootFolder()

    inputs = [os.path.abspath(p) for p in args["inputs"]]
    clips = [_import_clip(mp, root, p) for p in inputs]

    transition = args.get("transition")
    if transition:
        # Cross-dissolve transition between clips is not directly exposed via
        # CreateTimelineFromClips; requires manual timeline assembly + AddTransition.
        raise RuntimeError(
            "concat with transition not yet supported by davinci engine "
            "(requires per-item timeline assembly + AddTransition)"
        )

    timeline_name = f"mcp_concat_{os.getpid()}"
    timeline = mp.CreateTimelineFromClips(timeline_name, clips)
    if timeline is None:
        raise RuntimeError("CreateTimelineFromClips returned None")
    return {"output_path": f"timeline:{timeline_name}", "engine": "davinci"}


def cmd_extract_audio(args):
    """Extract audio: render the current timeline as audio-only."""
    resolve = _resolve()
    project = _project(resolve, os.environ.get("RESOLVE_PROJECT", "mcp-e2e"))
    timeline = _current_timeline(project)
    project.SetCurrentTimeline(timeline)

    fmt = args.get("format", "m4a")
    ext = {"mp3": "mp3", "wav": "wav"}.get(fmt, "m4a")
    codec = {"mp3": "MP3", "wav": "Linear PCM"}.get(fmt, "AAC")

    ws = _workspace()
    out_dir = str(ws / "davinci-out")
    Path(out_dir).mkdir(parents=True, exist_ok=True)
    out_name = f"audio_{os.getpid()}.{ext}"

    try:
        project.SetRenderSettings({
            "TargetDir": out_dir,
            "OutputFilename": out_name,
            "AudioCodec": codec,
            "ExportVideo": False,
            "ExportAudio": True,
        })
    except Exception as exc:
        raise RuntimeError(f"SetRenderSettings failed: {exc}")

    job_id = project.AddRenderJob()
    if not job_id:
        raise RuntimeError("AddRenderJob returned empty/None")
    if not project.StartRendering(job_id):
        raise RuntimeError("StartRendering failed")

    import time
    while project.IsRenderingInProgress():
        time.sleep(0.5)

    status = project.GetRenderJobStatus(job_id) or {}
    if status.get("JobStatus", "").lower() not in ("complete", "completed"):
        raise RuntimeError(f"audio render did not complete: {status}")

    return {"output_path": os.path.join(out_dir, out_name), "engine": "davinci"}


def cmd_cleanup_timeline(args):
    """Delete the current timeline (best-effort)."""
    resolve = _resolve()
    project = _project(resolve, os.environ.get("RESOLVE_PROJECT", "mcp-e2e"))
    tl = project.GetCurrentTimeline()
    if tl is None:
        return {"output_path": "", "engine": "davinci"}
    # DeleteCurrentTimeline is not in all API versions; guard.
    try:
        ok = project.DeleteCurrentTimeline()
    except Exception:
        ok = False
    return {"output_path": f"deleted:{tl.GetName()}", "engine": "davinci", "ok": bool(ok)}


COMMANDS = {
    "health": cmd_health,
    "cut": cmd_cut,
    "overlay": cmd_overlay,
    "burn_captions": cmd_burn_captions,
    "add_music": cmd_add_music,
    "render": cmd_render,
    "concat": cmd_concat,
    "extract_audio": cmd_extract_audio,
    "cleanup_timeline": cmd_cleanup_timeline,
}


def main():
    if len(sys.argv) < 2:
        print(json.dumps({"error": "missing command argument"}))
        sys.exit(2)
    command = sys.argv[1]
    handler = COMMANDS.get(command)
    if handler is None:
        print(json.dumps({"error": f"unknown command: {command}"}))
        sys.exit(2)

    raw = sys.stdin.read()
    try:
        args = json.loads(raw) if raw.strip() else {}
    except json.JSONDecodeError as exc:
        print(json.dumps({"error": f"invalid stdin json: {exc}"}))
        sys.exit(2)

    try:
        result = handler(args)
        if "engine" not in result:
            result["engine"] = "davinci"
        print(json.dumps(result))
    except Exception as exc:  # noqa: BLE001 — bridge must never crash without JSON
        print(json.dumps({"error": str(exc)}))
        sys.exit(1)


if __name__ == "__main__":
    main()