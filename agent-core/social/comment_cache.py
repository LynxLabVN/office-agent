"""Short-lived comment ref cache.

TikTok web exposes no stable public comment id and no comment permalink, so we
mint an opaque short token per comment and store the mapping (ref -> comment
identity) in a JSON cache under ``~/.hermes/social/comment_cache/<platform>.json``.
``social comments`` writes the cache; ``social reply`` reads it to locate the
comment by author+text. Refs EXPIRE (TTL below) — reply must run shortly after
comments.
"""

import json
import secrets
import time
from pathlib import Path
from typing import Optional

from .config import comment_cache_root

CACHE_TTL_SECONDS = 6 * 3600  # 6 hours


def _cache_file(platform: str) -> Path:
    return comment_cache_root() / f"{platform}.json"


def load(platform: str) -> dict:
    f = _cache_file(platform)
    if not f.exists():
        return {}
    try:
        data = json.loads(f.read_text(encoding="utf-8"))
    except Exception:
        return {}
    now = time.time()
    return {k: v for k, v in data.items() if now - v.get("ts", 0) < CACHE_TTL_SECONDS}


def save(platform: str, mapping: dict) -> None:
    _cache_file(platform).write_text(json.dumps(mapping, indent=2), encoding="utf-8")


def add(platform: str, video_url: str, comment: dict) -> str:
    """Mint a ref for ``comment`` and store it. Returns the ref token."""
    data = load(platform)
    ref = secrets.token_hex(4)
    data[ref] = {
        "video_url": video_url,
        "author": comment.get("author", ""),
        "text": comment.get("text", ""),
        "likes": comment.get("likes", ""),
        "ts": time.time(),
    }
    save(platform, data)
    return ref


def get(platform: str, ref: str) -> Optional[dict]:
    return load(platform).get(ref)