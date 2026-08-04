"""Resilient locator helpers + per-platform selector catalogs.

Each locator is a list of strategies tried in order; ``resolve`` chains them
with Playwright's ``Locator.or_`` and waits for the first to become visible
(or attached, for hidden file inputs). On total failure it dumps a screenshot
+ DOM snapshot to ``~/.hermes/social/diagnostics/`` and raises
``SelectorNotFound`` naming the step and every strategy tried — so a UI change
becomes a one-file patch guided by the dump, not an opaque traceback.

Strategy tuple shapes:
    ("css", selector)
    ("xpath", expr)                 # expr without the leading "xpath="
    ("role", role[, name])
    ("label", text)
    ("placeholder", text)
    ("text", text)
    ("testid", id)
"""

from .resolver import resolve, resolve_first, build_locator  # noqa: F401