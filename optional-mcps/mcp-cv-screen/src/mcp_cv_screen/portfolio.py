"""Portfolio analysis for web URLs and video files."""

from __future__ import annotations

from pathlib import Path
from typing import Any


def _fetch_url_text(url: str) -> str:
    try:
        import httpx
    except ImportError as exc:  # pragma: no cover
        raise RuntimeError("httpx is required to fetch URLs") from exc

    try:
        resp = httpx.get(url, timeout=30, follow_redirects=True)
        resp.raise_for_status()
        # Simple text extraction: prefer text if content-type is text/*.
        content_type = resp.headers.get("content-type", "").lower()
        if "text" in content_type:
            return resp.text
        return f"Fetched {url}: content-type={content_type}, length={len(resp.content)}"
    except Exception as exc:  # pragma: no cover
        return f"Failed to fetch {url}: {exc}"


def _transcribe_video(path: str, model_name: str | None = None) -> str | None:
    try:
        from faster_whisper import WhisperModel
    except ImportError:  # pragma: no cover
        return None

    model_name = model_name or "base"
    try:
        model = WhisperModel(model_name, device="cpu", compute_type="int8")
        segments, _ = model.transcribe(path)
        return " ".join(segment.text for segment in segments)
    except Exception as exc:  # pragma: no cover
        return f"Transcription failed: {exc}"


def analyze(
    url_or_files: list[str],
    *,
    whisper_model: str | None = None,
) -> dict[str, Any]:
    """Analyze portfolio URLs or local media files."""
    screenshots: list[str] = []
    text_parts: list[str] = []
    transcript: str | None = None

    video_extensions = {".mp4", ".mov", ".avi", ".mkv", ".webm"}

    for item in url_or_files:
        if item.startswith("http://") or item.startswith("https://"):
            text_parts.append(_fetch_url_text(item))
            screenshots.append(item)
        else:
            path = Path(item)
            if path.suffix.lower() in video_extensions:
                transcript = _transcribe_video(str(path), whisper_model)
                screenshots.append(str(path))
            elif path.exists():
                text_parts.append(f"Local file: {item}")
                screenshots.append(str(path))
            else:
                text_parts.append(f"Not found: {item}")

    return {
        "screenshots": screenshots,
        "text_extracted": "\n\n".join(text_parts).strip(),
        "transcript": transcript,
    }
