"""Facebook platform — v2 stub.

Facebook web-UI automation is harder than TikTok (more aggressive bot
detection, composer A/B tests, "Confirm your identity" challenges on new
profiles). Ship only after TikTok v1 is proven and the user has tested FB
login manually via ``social login facebook``. All methods raise
NotImplementedError in v1 with a clear pointer.
"""

from .base import Platform


class FacebookPlatform(Platform):
    name = "facebook"
    home_url = "https://www.facebook.com/"

    def _stub(self):
        raise NotImplementedError(
            "Facebook support is v2 (not in v1). TikTok is v1. "
            "Implement after TikTok is proven."
        )

    def login_check(self, page):
        self._stub()

    def do_login(self, page, log):
        self._stub()

    def publish(self, page, *, video_path, caption, privacy, tags, thumbnail,
                schedule, dry_run, log):
        self._stub()

    def list_comments(self, page, *, video_url, max, log):
        self._stub()

    def reply(self, page, *, ref, text, dry_run, log):
        self._stub()