"""Resolver: turn a list of strategies into the first matching Locator."""

import logging
from typing import Iterable

from playwright.sync_api import Locator, Page

from ..errors import SelectorNotFound
from ..logging_util import dump_failure


def _build(page: Page, strat: tuple) -> Locator:
    kind = strat[0]
    if kind == "css":
        return page.locator(strat[1])
    if kind == "xpath":
        return page.locator("xpath=" + strat[1])
    if kind == "role":
        role = strat[1]
        name = strat[2] if len(strat) > 2 else None
        return page.get_by_role(role, name=name) if name else page.get_by_role(role)
    if kind == "label":
        return page.get_by_label(strat[1])
    if kind == "placeholder":
        return page.get_by_placeholder(strat[1])
    if kind == "text":
        return page.get_by_text(strat[1])
    if kind == "testid":
        return page.get_by_test_id(strat[1])
    raise ValueError(f"unknown strategy kind: {kind!r}")


def _strat_name(s: tuple) -> str:
    if s[0] in ("role",):
        return f"{s[0]}:{s[1]}"
    return f"{s[0]}:{s[1]}" if len(s) > 1 else s[0]


def resolve(
    page: Page,
    strategies: Iterable[tuple],
    *,
    timeout_ms: int = 20_000,
    flow: str = "?",
    step: str = "?",
    state: str = "visible",
    scope=None,
) -> Locator:
    """Return the first locator among ``strategies`` that attaches/visibles.

    ``page`` is used for diagnostics dumps. ``scope`` (optional) is the element
    the strategies are resolved against (defaults to ``page``) — pass a Locator
    to search within a sub-element (e.g. a single comment node).

    ``state``: "visible" (default) for normal elements, "attached" for hidden
    inputs (file inputs are often display:none).
    """
    log = logging.getLogger("social.selectors")
    strats = list(strategies)
    if not strats:
        dump_failure(page, flow, step)
        raise SelectorNotFound(step=step, tried=[])

    root = scope if scope is not None else page
    combined = None
    for s in strats:
        loc = _build(root, s)
        combined = loc if combined is None else combined.or_(loc)
    first = combined.first
    try:
        first.wait_for(state=state, timeout=timeout_ms)
        return first
    except Exception:
        dump_failure(page, flow, step)
        log.warning("resolve failed flow=%s step=%s tried=%s", flow, step,
                    [_strat_name(s) for s in strats])
        raise SelectorNotFound(step=step, tried=[_strat_name(s) for s in strats])


def build_locator(scope, strat: tuple) -> Locator:
    """Public alias of :func:`_build` for ad-hoc locator construction."""
    return _build(scope, strat)


def resolve_first(page: Page, strategies: Iterable[tuple], **kw) -> Locator:
    """Alias for :func:`resolve`."""
    return resolve(page, strategies, **kw)