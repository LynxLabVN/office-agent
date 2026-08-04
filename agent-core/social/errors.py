"""Error hierarchy for the social tool."""


class SocialError(Exception):
    """Base for all social-tool errors."""


class NotLoggedIn(SocialError):
    """Account is not logged in at all."""


class SessionExpired(SocialError):
    """Profile session has expired; user must re-run ``social login``."""


class LoginChallenge(SocialError):
    """A captcha / 2FA / verification challenge appeared during an action.

    v1 does not solve challenges; abort and instruct manual ``social login``.
    """


class SelectorNotFound(SocialError):
    """A resilient locator exhausted all strategies. Diagnostics dumped."""

    def __init__(self, step: str, tried=None, diag=None):
        self.step = step
        self.tried = tried or []
        self.diag = diag
        super().__init__(
            f"selector not found for step={step!r}; tried={self.tried}. "
            f"See diagnostics dump in ~/.hermes/social/diagnostics/."
        )


class UploadFailed(SocialError):
    """Video upload did not complete within the timeout."""


class CommentNotFound(SocialError):
    """A comment ref did not match any loaded comment (or expired)."""

    def __init__(self, ref: str):
        self.ref = ref
        super().__init__(
            f"comment ref {ref!r} not found. Refs expire (minutes-to-hours) — "
            f"re-run `comments` then `reply` immediately."
        )


class PlatformNotSupported(SocialError):
    """Requested platform is not implemented in this version."""