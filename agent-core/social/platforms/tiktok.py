"""TikTok platform — v1 implementation.

Drives the TikTok web UI (creator-center upload + video comment pages) as a
logged-in normal account via a persistent Chrome profile. Flows:

  * login_check — detect logged-in vs Log-in button vs challenge
  * do_login   — headed, manual login; profile persists cookies
  * publish    — set_input_files on the upload composer → caption → Post
  * list_comments — scroll-load + scrape; mint short-lived refs via comment_cache
  * reply      — re-locate comment by author+text, open reply input, submit

Selectors live in ``selectors/tiktok.py`` and WILL break when TikTok changes
its UI — patch that one file using the diagnostics dumps in
``~/.hermes/social/diagnostics/``.
"""

import logging
import time
from pathlib import Path

from .. import comment_cache
from ..config import UPLOAD_TIMEOUT_MS
from ..dryrun import DryRunGate
from ..errors import CommentNotFound, LoginChallenge, SelectorNotFound, SessionExpired, UploadFailed
from ..selectors import build_locator, resolve
from ..selectors import tiktok as sel
from .base import Platform


# privacy arg value -> visible option text in the TikTok "Who can watch" dropdown
PRIVACY_MAP = {"public": "Everyone", "followers": "Followers", "friends": "Friends"}


def _any_visible(scope, strategies, timeout_ms=2500):
    """Lightweight probe: True if any strategy becomes visible within timeout.

    No diagnostics dump (used for login-state checks, not action steps).
    """
    for s in strategies:
        try:
            loc = build_locator(scope, s)
            loc.first.wait_for(state="visible", timeout=timeout_ms)
            return True
        except Exception:
            continue
    return False


def _child_text(item, strategies, timeout_ms=2000):
    """Best-effort inner_text of the first matching child strategy; "" on miss."""
    for s in strategies:
        try:
            loc = build_locator(item, s)
            txt = loc.first.inner_text(timeout=timeout_ms)
            if txt and txt.strip():
                return txt.strip()
        except Exception:
            continue
    return ""


class TikTokPlatform(Platform):
    name = "tiktok"
    home_url = "https://www.tiktok.com/"

    # ---- login state ----------------------------------------------------

    def login_check(self, page) -> None:
        log = logging.getLogger("social.tiktok")
        # challenge takes priority
        if _any_visible(page, sel.LOGIN_CHALLENGE_MARKERS, timeout_ms=2000):
            raise LoginChallenge(
                "TikTok is showing a verification/challenge screen. Run "
                "`python -m social login tiktok` (headed) and clear it manually, then retry."
            )
        if _any_visible(page, sel.LOGIN_BUTTON, timeout_ms=2500):
            raise SessionExpired(
                "TikTok session expired (Log in button visible). Run "
                "`python -m social login tiktok` (headed) and complete login manually, then retry."
            )
        if not _any_visible(page, sel.LOGGED_IN_MARKERS, timeout_ms=2500):
            # page may still be loading; give it one more moment then fail safe
            page.wait_for_timeout(2000)
            if not _any_visible(page, sel.LOGGED_IN_MARKERS, timeout_ms=2500):
                raise SessionExpired(
                    "Cannot confirm TikTok login state. Run "
                    "`python -m social login tiktok` (headed) and log in, then retry."
                )
        log.debug("login_check OK")

    def do_login(self, page, log) -> None:
        log.info("Opening TikTok for manual login (headed).")
        page.goto(self.home_url, wait_until="domcontentloaded", timeout=60_000)
        print(
            "\nA Chrome window opened on TikTok.\n"
            "Log in there (phone/email/QR/2FA all fine). When your avatar is visible\n"
            "in the window, come back here and press Enter.\n"
        )
        try:
            input("Press Enter once you are logged in... ")
        except EOFError:
            pass
        # confirm
        try:
            self.login_check(page)
        except (SessionExpired, LoginChallenge) as e:
            log.warning("login check after manual login: %s", e)
        log.info("Login saved to %s", Path.home() / ".hermes" / "social" / "profiles" / self.name)

    # ---- publish --------------------------------------------------------

    def publish(self, page, *, video_path, caption, privacy=None, tags=None,
                thumbnail=None, schedule=None, dry_run: DryRunGate, log) -> dict:
        vp = Path(video_path).expanduser().resolve()
        if not vp.is_file():
            raise UploadFailed(f"video file not found: {vp}")
        self.login_check(page)

        # navigate to an upload page (try legacy /upload first; TikTok redirects)
        last_err = None
        for url in sel.UPLOAD_PAGE_URLS:
            try:
                page.goto(url, wait_until="domcontentloaded", timeout=60_000)
                break
            except Exception as e:  # noqa: BLE001
                last_err = e
        else:
            raise UploadFailed(f"could not open upload page: {last_err}")
        page.wait_for_timeout(2000)

        # attach the local video file (file input is usually hidden -> attached)
        file_input = resolve(
            page, sel.UPLOAD_FILE_INPUT, state="attached",
            timeout_ms=30_000, flow="publish", step="upload_file_input",
        )
        file_input.set_input_files(str(vp))
        log.info("file attached; waiting for upload + processing (up to %ds)...",
                 UPLOAD_TIMEOUT_MS // 1000)

        # wait for the caption editor to appear = upload/processing done
        try:
            resolve(page, sel.CAPTION_EDITOR, state="visible",
                    timeout_ms=UPLOAD_TIMEOUT_MS, flow="publish", step="caption_editor_appear")
        except SelectorNotFound:
            raise UploadFailed(
                "caption editor did not appear in time; upload may have failed. "
                "Check ~/.hermes/social/diagnostics/."
            )

        # fill caption (+ tags appended as #tag)
        editor = resolve(page, sel.CAPTION_EDITOR, state="visible",
                        timeout_ms=15_000, flow="publish", step="caption_editor_focus")
        editor.click()
        full_caption = caption or ""
        if tags:
            tagstr = " ".join("#" + t.lstrip("#") for t in tags if t)
            full_caption = (full_caption + " " + tagstr).strip()
        if full_caption:
            page.keyboard.type(full_caption, delay=80)
            log.info("caption filled (%d chars)", len(full_caption))

        # privacy (optional; non-fatal)
        target = PRIVACY_MAP.get((privacy or "").lower()) if privacy else None
        if target:
            try:
                dd = resolve(page, sel.PRIVACY_DROPDOWN, state="visible",
                             timeout_ms=15_000, flow="publish", step="privacy_dropdown")
                dd.click()
                page.wait_for_timeout(800)
                page.get_by_text(target, exact=False).first.click()
                log.info("privacy set to %s", target)
            except Exception as e:  # noqa: BLE001
                log.warning("could not set privacy=%s: %s (continuing with default)", privacy, e)

        # thumbnail / schedule are accepted by the CLI but not wired in v1
        if thumbnail:
            log.info("thumbnail ignored in v1 (not wired)")
        if schedule:
            log.info("schedule ignored in v1 (not wired)")

        # Post (or stop for dry-run)
        if dry_run.would_submit("click Post", log, page):
            return {"platform": self.name, "status": "dry_run", "video_url": None,
                    "post_id": "unknown"}

        post_btn = resolve(page, sel.POST_BUTTON, state="visible",
                            timeout_ms=15_000, flow="publish", step="post_button")
        post_btn.click()
        log.info("Post clicked; waiting for confirmation...")

        # success: URL navigates to /@<handle>/video/<id>
        try:
            page.wait_for_url(lambda u: sel.POST_SUCCESS_URL_FRAGMENT in u, timeout=120_000)
            log.info("published: %s", page.url)
            return {"platform": self.name, "status": "published", "video_url": page.url}
        except Exception:
            if _any_visible(page, sel.POST_SUCCESS_MARKERS, timeout_ms=5000):
                return {"platform": self.name, "status": "published", "video_url": page.url}
            return {"platform": self.name, "status": "unknown",
                    "video_url": page.url, "post_id": "unknown"}

    # ---- comments -------------------------------------------------------

    def list_comments(self, page, *, video_url, max, log) -> list:
        url = sel.normalize_video_url(video_url)
        self.login_check(page)
        page.goto(url, wait_until="domcontentloaded", timeout=60_000)
        container = resolve(page, sel.COMMENTS_CONTAINER, state="visible",
                             timeout_ms=30_000, flow="comments", step="container")

        results = []
        seen = set()
        no_new_rounds = 0
        rounds = 0
        max_rounds = 40
        while len(results) < max and rounds < max_rounds:
            rounds += 1
            before = len(results)
            item_locs = build_locator(container, sel.COMMENT_ITEM[0])
            count = item_locs.count()
            for i in range(count):
                if len(results) >= max:
                    break
                item = item_locs.nth(i)
                author = _child_text(item, sel.COMMENT_AUTHOR)
                text = _child_text(item, sel.COMMENT_TEXT)
                if not text:
                    continue
                key = (author, text[:60])
                if key in seen:
                    continue
                seen.add(key)
                likes = _child_text(item, sel.COMMENT_LIKES)
                ref = comment_cache.add(self.name, url,
                                        {"author": author, "text": text, "likes": likes})
                results.append({"comment_ref": ref, "author": author,
                                "text": text, "likes": likes})
            if len(results) == before:
                no_new_rounds += 1
                if no_new_rounds >= 3:
                    break
            else:
                no_new_rounds = 0
            # scroll to load more
            try:
                container.evaluate("el => { el.scrollTop = el.scrollHeight; }")
            except Exception:
                page.keyboard.press("PageDown")
            page.wait_for_timeout(1500)
        log.info("collected %d comments (refs cached, TTL %dh)",
                 len(results), comment_cache.CACHE_TTL_SECONDS // 3600)
        return results

    # ---- reply ----------------------------------------------------------

    def _find_comment_item(self, container, page, want_author, want_text, log):
        """Scroll-load comments; return the Locator whose author+text matches."""
        seen = 0
        rounds = 0
        max_rounds = 40
        while rounds < max_rounds:
            rounds += 1
            item_locs = build_locator(container, sel.COMMENT_ITEM[0])
            count = item_locs.count()
            for i in range(seen, count):
                item = item_locs.nth(i)
                author = _child_text(item, sel.COMMENT_AUTHOR)
                text = _child_text(item, sel.COMMENT_TEXT)
                if not text:
                    continue
                if author == want_author and text.startswith(want_text[:40]):
                    return item
            seen = count
            try:
                container.evaluate("el => { el.scrollTop = el.scrollHeight; }")
            except Exception:
                page.keyboard.press("PageDown")
            page.wait_for_timeout(1500)
        return None

    def reply(self, page, *, ref, text, dry_run: DryRunGate, log) -> dict:
        entry = comment_cache.get(self.name, ref)
        if not entry:
            raise CommentNotFound(ref)
        video_url = entry["video_url"]
        want_author = entry.get("author", "")
        want_text = entry.get("text", "")
        self.login_check(page)
        page.goto(video_url, wait_until="domcontentloaded", timeout=60_000)
        container = resolve(page, sel.COMMENTS_CONTAINER, state="visible",
                             timeout_ms=30_000, flow="reply", step="container")

        item = self._find_comment_item(container, page, want_author, want_text, log)
        if item is None:
            raise CommentNotFound(ref)

        reply_btn = resolve(page, sel.REPLY_BUTTON, scope=item, state="visible",
                            timeout_ms=10_000, flow="reply", step="reply_button")
        reply_btn.click()
        log.info("reply opened for comment by %s", want_author)

        reply_input = resolve(page, sel.REPLY_INPUT, state="visible",
                              timeout_ms=15_000, flow="reply", step="reply_input")
        reply_input.click()
        page.keyboard.type(text, delay=80)

        if dry_run.would_submit("send reply", log, page):
            return {"platform": self.name, "status": "dry_run"}

        submit = resolve(page, sel.REPLY_SUBMIT, state="visible",
                         timeout_ms=15_000, flow="reply", step="reply_submit")
        submit.click()
        page.wait_for_timeout(2500)
        log.info("reply sent to comment by %s", want_author)
        return {"platform": self.name, "status": "replied"}