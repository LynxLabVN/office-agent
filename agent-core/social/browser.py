"""Playwright bootstrap: persistent Chrome context per platform.

The persistent ``user_data_dir`` is the core of the "drive my own browser as a
normal account" design — cookies and login state live in a real Chrome profile
under ``~/.hermes/social/profiles/<platform>/``. One-time manual headed login
(via ``social login <platform>``) populates it; subsequent actions reuse it.
"""

import logging
from contextlib import contextmanager

from playwright.sync_api import BrowserContext, sync_playwright

from .config import DEFAULT_CHANNEL, profile_dir


@contextmanager
def session(
    platform: str,
    *,
    headed: bool = True,
    slowmo: int = 0,
    channel: str | None = None,
    profile_dir_override: str | None = None,
):
    """Yield a persistent BrowserContext for ``platform``.

    Closes the context and stops playwright on exit. The context already has
    one open page available as ``ctx.pages[0]``. ``profile_dir_override`` (from
    ``--profile-dir``) replaces the default ``~/.hermes/social/profiles/<platform>``.
    """
    log = logging.getLogger("social.browser")
    if profile_dir_override:
        from pathlib import Path
        ud = Path(profile_dir_override).expanduser()
        ud.mkdir(parents=True, exist_ok=True)
    else:
        ud = profile_dir(platform)
    ch = channel or DEFAULT_CHANNEL
    pw = sync_playwright().start()
    try:
        ctx = pw.chromium.launch_persistent_context(
            user_data_dir=str(ud),
            channel=ch if ch and ch != "chromium" else None,
            headless=not headed,
            slow_mo=slowmo,
            viewport={"width": 1280, "height": 800},
            args=["--disable-blink-features=AutomationControlled"],
        )
        log.info("launched persistent context: platform=%s headless=%s profile=%s",
                 platform, not headed, ud)
        try:
            yield ctx
        finally:
            try:
                ctx.close()
            except Exception:
                pass
    finally:
        pw.stop()


def first_page(ctx: BrowserContext):
    """Return the context's existing page, or create one."""
    if ctx.pages:
        return ctx.pages[0]
    return ctx.new_page()