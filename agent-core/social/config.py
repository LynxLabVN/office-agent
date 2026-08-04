"""Paths, defaults, and env overrides for the social tool.

No imports from the deleted ``hermes_constants`` — uses its own ``SOCIAL_HOME``
env var (default ``~/.hermes`` to mirror the repo's existing convention).
"""

import os
from pathlib import Path

VERSION = "0.1.0"


def home_dir() -> Path:
    """Root for all social-tool state. Override with ``SOCIAL_HOME``."""
    return Path(os.environ.get("SOCIAL_HOME", str(Path.home() / ".hermes")))


def _ensure(p: Path) -> Path:
    p.mkdir(parents=True, exist_ok=True)
    return p


def profiles_root() -> Path:
    return _ensure(home_dir() / "social" / "profiles")


def profile_dir(platform: str) -> Path:
    return _ensure(profiles_root() / platform)


def screenshots_root() -> Path:
    return _ensure(home_dir() / "social" / "screenshots")


def diagnostics_root() -> Path:
    return _ensure(home_dir() / "social" / "diagnostics")


def comment_cache_root() -> Path:
    return _ensure(home_dir() / "social" / "comment_cache")


# Browser channel: "chromium" (Playwright bundled) | "chrome" | "msedge".
# On this WSL box there is no system Chrome, so bundled Chromium is the only
# working option today. The persistent user_data_dir — not the channel — is
# what delivers the logged-in-normal-account feel.
DEFAULT_CHANNEL = os.environ.get("SOCIAL_CHANNEL", "chromium")

DEFAULT_TIMEOUT_MS = 60_000        # generic waits
UPLOAD_TIMEOUT_MS = 300_000        # video upload + processing can take minutes
COMMENT_SCROLL_TIMEOUT_MS = 20_000

# Low-volume guidance (documented, not enforced).
RATE_GUIDE_POSTS_PER_DAY = 5
RATE_GUIDE_REPLIES_PER_DAY = 20