"""Logging setup + on-failure diagnostics (screenshot + DOM dump)."""

import logging
import time
from pathlib import Path
from typing import Optional

from .config import diagnostics_root


def setup_logging(level: str = "INFO") -> logging.Logger:
    logging.basicConfig(
        level=getattr(logging, level.upper(), logging.INFO),
        format="%(asctime)s %(levelname)s %(name)s: %(message)s",
    )
    return logging.getLogger("social")


def dump_failure(page, flow: str, step: str) -> Optional[Path]:
    """Write a full-page screenshot + DOM HTML to the diagnostics dir.

    Best-effort: never raises (a failure during diagnostics must not mask the
    original error). Returns the base path (without extension) or None.
    """
    try:
        ts = time.strftime("%Y%m%d-%H%M%S")
        base = diagnostics_root() / f"{ts}-{flow}-{step}"
        try:
            page.screenshot(path=str(base.with_suffix(".png")), full_page=True)
        except Exception:
            pass
        try:
            base.with_suffix(".html").write_text(page.content(), encoding="utf-8")
        except Exception:
            pass
        return base
    except Exception:
        return None