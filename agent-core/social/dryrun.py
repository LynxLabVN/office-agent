"""Dry-run gate: walks every flow step except the final submit.

``set_input_files`` IS allowed in dry-run (so the user can confirm the upload
actually starts and selectors resolve); only the final mutating submit
(Post / Reply send) is skipped, with a screenshot taken at that point.
"""

import logging
from pathlib import Path
from typing import Optional

from .config import screenshots_root


class DryRunGate:
    def __init__(self, enabled: bool = False):
        self.enabled = enabled

    def would_submit(self, label: str, log: logging.Logger, page=None) -> bool:
        """Return True if the caller should SKIP the submit (because dry-run).

        In dry-run: screenshot + log + return True (skip).
        Otherwise: return False (caller proceeds to submit).
        """
        if not self.enabled:
            return False
        log.info("[dry-run] would %s", label)
        if page is not None:
            try:
                import time as _t
                shot = screenshots_root() / f"{_t.strftime('%Y%m%d-%H%M%S')}-dryrun-{label.replace(' ', '-')}.png"
                page.screenshot(path=str(shot), full_page=True)
                log.info("[dry-run] screenshot: %s", shot)
            except Exception:
                pass
        return True