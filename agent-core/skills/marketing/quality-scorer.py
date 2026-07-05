#!/usr/bin/env python3
"""CLI entry point for the auto-edit quality scorer.

The importable implementation lives in ``quality_scorer.py`` (underscore)
because Python module names cannot contain hyphens.
"""

from __future__ import annotations

try:
    from skills.marketing.quality_scorer import main
except ImportError:
    from quality_scorer import main  # type: ignore[no-redef]

if __name__ == "__main__":
    main()
