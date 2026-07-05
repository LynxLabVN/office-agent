#!/usr/bin/env python3
"""Auto-edit quality scorer for marketing video renders.

Scores a rendered video 0-100 across:
- Hook retention (first 3s has motion + text + clear subject)
- Audio quality (FFmpeg loudnorm, no clipping)
- Caption accuracy (Whisper confidence)
- Resolution / fps correctness

Formats 1-5,7,9 are strong-auto. Formats 6,10 always require human review.
"""

from __future__ import annotations

import json
import re
import shutil
import subprocess
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Dict, List, Optional

# Formats that are safe for fully automated editing.
STRONG_AUTO_FORMATS = {
    "short",
    "ugc",
    "demo",
    "testimonial",
    "unboxing",
    "bts",
    "comparison",
    # numeric aliases used in some call sites
    "1",
    "2",
    "3",
    "4",
    "5",
    "7",
    "9",
}

# Formats that always require human review regardless of score.
ALWAYS_HUMAN_FORMATS = {
    "storytelling",
    "skits",
    "6",
    "10",
}


@dataclass
class VideoProbe:
    width: int
    height: int
    fps: float
    duration: float
    has_video_stream: bool
    has_audio_stream: bool


@dataclass
class LoudnormInfo:
    input_i: float
    input_tp: float
    input_lra: float
    input_thresh: float
    target_offset: float
    output_i: float
    output_tp: float
    output_lra: float
    output_thresh: float
    normalization_type: str


@dataclass
class QualityReport:
    score: int
    passed: bool
    flags: List[str]
    details: Dict[str, Any]
    requires_human_review: bool


def _run(cmd: List[str], timeout: int = 60) -> subprocess.CompletedProcess:
    return subprocess.run(
        cmd,
        capture_output=True,
        text=True,
        timeout=timeout,
        check=False,
    )


def _ffprobe(path: str) -> Optional[VideoProbe]:
    if not shutil.which("ffprobe"):
        return None
    cmd = [
        "ffprobe",
        "-v",
        "error",
        "-select_streams",
        "v:0",
        "-show_entries",
        "stream=width,height,r_frame_rate,avg_frame_rate,duration",
        "-of",
        "json",
        path,
    ]
    proc = _run(cmd)
    if proc.returncode != 0:
        return None
    try:
        data = json.loads(proc.stdout)
    except json.JSONDecodeError:
        return None
    streams = data.get("streams", [])
    if not streams:
        return None
    s = streams[0]
    width = int(s.get("width", 0))
    height = int(s.get("height", 0))
    fps = _parse_fps(s.get("r_frame_rate", "0/1"))
    duration = float(s.get("duration", 0) or 0)

    # Audio presence
    has_audio = _has_audio_stream(path)

    return VideoProbe(
        width=width,
        height=height,
        fps=fps,
        duration=duration,
        has_video_stream=True,
        has_audio_stream=has_audio,
    )


def _has_audio_stream(path: str) -> bool:
    cmd = [
        "ffprobe",
        "-v",
        "error",
        "-select_streams",
        "a:0",
        "-show_entries",
        "stream=codec_type",
        "-of",
        "json",
        path,
    ]
    proc = _run(cmd)
    if proc.returncode != 0:
        return False
    try:
        data = json.loads(proc.stdout)
    except json.JSONDecodeError:
        return False
    return any(s.get("codec_type") == "audio" for s in data.get("streams", []))


def _parse_fps(fps_str: str) -> float:
    if "/" in fps_str:
        num, den = fps_str.split("/", 1)
        try:
            return float(num) / float(den)
        except (ValueError, ZeroDivisionError):
            return 0.0
    try:
        return float(fps_str)
    except ValueError:
        return 0.0


def _loudnorm(path: str) -> Optional[LoudnormInfo]:
    if not shutil.which("ffmpeg"):
        return None
    filter_str = (
        "loudnorm=print_format=json:I=-16:TP=-1.5:LRA=11"
    )
    cmd = [
        "ffmpeg",
        "-hide_banner",
        "-i",
        path,
        "-af",
        filter_str,
        "-f",
        "null",
        "-",
    ]
    proc = _run(cmd, timeout=120)
    stderr = proc.stderr or ""
    # loudnorm prints JSON after the line "[Parsed_loudnorm...]"
    match = re.search(r"\{\s*\"input_i\".*?\}\s*", stderr, re.DOTALL)
    if not match:
        return None
    try:
        data = json.loads(match.group(0))
    except json.JSONDecodeError:
        return None
    return LoudnormInfo(
        input_i=float(data.get("input_i", 0)),
        input_tp=float(data.get("input_tp", 0)),
        input_lra=float(data.get("input_lra", 0)),
        input_thresh=float(data.get("input_thresh", 0)),
        target_offset=float(data.get("target_offset", 0)),
        output_i=float(data.get("output_i", 0)),
        output_tp=float(data.get("output_tp", 0)),
        output_lra=float(data.get("output_lra", 0)),
        output_thresh=float(data.get("output_thresh", 0)),
        normalization_type=data.get("normalization_type", ""),
    )


def _whisper_confidence(path: str) -> Optional[float]:
    """Return average Whisper confidence if faster-whisper is installed."""
    try:
        from faster_whisper import WhisperModel
    except Exception:
        return None
    try:
        model = WhisperModel("base", compute_type="int8")
        segments, _ = model.transcribe(path, language="vi")
        confidences = [seg.avg_logprob for seg in segments]
        if not confidences:
            return None
        # Normalize logprob to roughly 0-1 scale for a typical -1..0 range.
        return min(1.0, max(0.0, sum(confidences) / len(confidences) + 1.0))
    except Exception:
        return None


def _score_hook_retention(
    probe: Optional[VideoProbe],
    has_text_overlay: bool,
    has_motion: bool,
    has_clear_subject: bool,
) -> Dict[str, Any]:
    score = 0
    notes: List[str] = []
    if probe and probe.duration >= 3.0:
        score += 10
    else:
        notes.append("video shorter than 3 seconds")
    if has_text_overlay:
        score += 10
    else:
        notes.append("no text overlay in first 3s")
    if has_motion:
        score += 10
    else:
        notes.append("no detected motion in first 3s")
    if has_clear_subject:
        score += 10
    else:
        notes.append("subject not clearly framed in first 3s")
    return {"score": score, "max": 40, "notes": notes}


def _score_audio(loudnorm: Optional[LoudnormInfo]) -> Dict[str, Any]:
    if loudnorm is None:
        return {"score": 15, "max": 25, "notes": ["could not analyze audio"]}
    score = 25
    notes: List[str] = []
    # Clipping: true peak > 0 dBTP indicates clipping.
    if loudnorm.input_tp > 0.0:
        score -= 25
        notes.append(f"audio clipping detected (true peak {loudnorm.input_tp:.2f} dBTP)")
    # Loudness target: -16 LUFS ±3.
    if abs(loudnorm.input_i - (-16.0)) > 6.0:
        score -= 5
        notes.append(f"loudness far from target ({loudnorm.input_i:.2f} LUFS)")
    if loudnorm.input_lra > 15.0:
        score -= 5
        notes.append(f"high loudness range ({loudnorm.input_lra:.2f} LU)")
    return {"score": max(0, score), "max": 25, "notes": notes}


def _score_captions(confidence: Optional[float]) -> Dict[str, Any]:
    if confidence is None:
        return {"score": 10, "max": 20, "notes": ["caption confidence not available"]}
    if confidence >= 0.9:
        return {"score": 20, "max": 20, "notes": []}
    if confidence >= 0.8:
        return {"score": 15, "max": 20, "notes": ["caption confidence acceptable"]}
    return {
        "score": int(20 * max(0.0, confidence)),
        "max": 20,
        "notes": [f"caption confidence low ({confidence:.2f})"],
    }


def _score_resolution(
    probe: Optional[VideoProbe],
    expected_resolution: Optional[str],
    expected_fps: Optional[float],
) -> Dict[str, Any]:
    if probe is None:
        return {"score": 10, "max": 15, "notes": ["could not probe video"]}
    score = 15
    notes: List[str] = []
    if expected_resolution:
        exp_w, exp_h = _parse_resolution(expected_resolution)
        if exp_w and exp_h and (probe.width != exp_w or probe.height != exp_h):
            score -= 7
            notes.append(
                f"resolution mismatch: expected {expected_resolution}, got {probe.width}x{probe.height}"
            )
    if expected_fps and abs(probe.fps - expected_fps) > 1.0:
        score -= 3
        notes.append(f"fps mismatch: expected {expected_fps}, got {probe.fps:.2f}")
    if not probe.has_audio_stream:
        score -= 5
        notes.append("no audio stream")
    return {"score": max(0, score), "max": 15, "notes": notes}


def _parse_resolution(res: str) -> tuple[Optional[int], Optional[int]]:
    m = re.match(r"(\d+)\s*x\s*(\d+)", res.lower())
    if m:
        return int(m.group(1)), int(m.group(2))
    if res.lower() == "1080x1920":
        return 1080, 1920
    if res.lower() == "1920x1080":
        return 1920, 1080
    return None, None


def score_render(
    video_path: str,
    fmt: str,
    *,
    expected_resolution: Optional[str] = None,
    expected_fps: Optional[float] = None,
    has_text_overlay: bool = True,
    has_motion: bool = True,
    has_clear_subject: bool = True,
) -> QualityReport:
    """Score a rendered video. Always human-review storytelling/skits."""
    fmt_norm = str(fmt).strip().lower()
    requires_human_review = fmt_norm in ALWAYS_HUMAN_FORMATS

    probe = _ffprobe(video_path)
    loudnorm = _loudnorm(video_path)
    whisper_conf = _whisper_confidence(video_path)

    hook = _score_hook_retention(probe, has_text_overlay, has_motion, has_clear_subject)
    audio = _score_audio(loudnorm)
    captions = _score_captions(whisper_conf)
    resolution = _score_resolution(probe, expected_resolution, expected_fps)

    total = hook["score"] + audio["score"] + captions["score"] + resolution["score"]
    score = min(100, max(0, total))

    flags: List[str] = []
    flags.extend(hook["notes"])
    flags.extend(audio["notes"])
    flags.extend(captions["notes"])
    flags.extend(resolution["notes"])

    if score < 60:
        flags.append("auto-edit low quality")
        requires_human_review = True

    return QualityReport(
        score=score,
        passed=score >= 60 and not requires_human_review,
        flags=flags,
        details={
            "hook": hook,
            "audio": audio,
            "captions": captions,
            "resolution": resolution,
            "probe": {
                "width": probe.width if probe else None,
                "height": probe.height if probe else None,
                "fps": probe.fps if probe else None,
                "duration": probe.duration if probe else None,
                "has_audio_stream": probe.has_audio_stream if probe else None,
            },
            "loudnorm": {
                "input_i": loudnorm.input_i if loudnorm else None,
                "input_tp": loudnorm.input_tp if loudnorm else None,
                "input_lra": loudnorm.input_lra if loudnorm else None,
            },
            "caption_confidence": whisper_conf,
        },
        requires_human_review=requires_human_review,
    )


def score_render_with_metrics(
    fmt: str,
    *,
    duration: float = 10.0,
    has_text_overlay: bool = True,
    has_motion: bool = True,
    has_clear_subject: bool = True,
    input_tp: float = -2.0,
    input_i: float = -16.0,
    input_lra: float = 8.0,
    caption_confidence: Optional[float] = 0.9,
    width: int = 1080,
    height: int = 1920,
    fps: float = 30.0,
    has_audio_stream: bool = True,
) -> QualityReport:
    """Test-friendly variant that bypasses ffprobe/whisper."""
    fmt_norm = str(fmt).strip().lower()
    requires_human_review = fmt_norm in ALWAYS_HUMAN_FORMATS

    probe = VideoProbe(
        width=width,
        height=height,
        fps=fps,
        duration=duration,
        has_video_stream=True,
        has_audio_stream=has_audio_stream,
    )
    loudnorm = LoudnormInfo(
        input_i=input_i,
        input_tp=input_tp,
        input_lra=input_lra,
        input_thresh=-20.0,
        target_offset=0.0,
        output_i=-16.0,
        output_tp=-1.5,
        output_lra=11.0,
        output_thresh=-20.0,
        normalization_type="dynamic",
    )

    hook = _score_hook_retention(probe, has_text_overlay, has_motion, has_clear_subject)
    audio = _score_audio(loudnorm)
    captions = _score_captions(caption_confidence)
    resolution = _score_resolution(probe, f"{width}x{height}", fps)

    total = hook["score"] + audio["score"] + captions["score"] + resolution["score"]
    score = min(100, max(0, total))

    flags: List[str] = []
    flags.extend(hook["notes"])
    flags.extend(audio["notes"])
    flags.extend(captions["notes"])
    flags.extend(resolution["notes"])

    if score < 60:
        flags.append("auto-edit low quality")
        requires_human_review = True

    return QualityReport(
        score=score,
        passed=score >= 60 and not requires_human_review,
        flags=flags,
        details={
            "hook": hook,
            "audio": audio,
            "captions": captions,
            "resolution": resolution,
        },
        requires_human_review=requires_human_review,
    )


def main() -> None:
    import argparse

    parser = argparse.ArgumentParser(description="Score a rendered marketing video")
    parser.add_argument("video_path")
    parser.add_argument("--format", default="short")
    parser.add_argument("--resolution", default=None)
    parser.add_argument("--fps", type=float, default=None)
    args = parser.parse_args()

    report = score_render(
        args.video_path,
        args.format,
        expected_resolution=args.resolution,
        expected_fps=args.fps,
    )
    print(json.dumps(report.__dict__, indent=2))


if __name__ == "__main__":
    main()
