"""TikTok resilient locator catalog.

This is the file that breaks when TikTok changes its web UI and gets patched
using the diagnostics dumps in ``~/.hermes/social/diagnostics/``. Each locator
is a list of strategies tried in order (see ``resolver.resolve``).

Selectors are best-effort against the current TikTok web UI (creator-center
upload + video comment pages). They WILL need live tuning — run with
``--headed --slowmo 200`` to watch, and inspect the dumped HTML/screenshot on
any ``SelectorNotFound``.
"""

# --- Login state ----------------------------------------------------------

# Any one present => logged in.
LOGGED_IN_MARKERS = [
    ("css", "[data-e2e='upload-icon']"),
    ("css", "[data-e2e='profile-icon']"),
    ("css", "[data-e2e='avatar']"),
    ("xpath", "//div[contains(@data-e2e,'avatar')]"),
    ("xpath", "//button[contains(@data-e2e,'upload')]"),
]

# Presence => logged OUT (takes priority over LOGGED_IN_MARKERS).
LOGIN_BUTTON = [
    ("css", "[data-e2e='login-link']"),
    ("xpath", "//a[contains(@data-e2e,'login')]"),
    ("text", "Log in"),
]

# Challenge / verification interstitials -> abort to manual login.
LOGIN_CHALLENGE_MARKERS = [
    ("text", "Verify you are human"),
    ("text", "suspicious"),
    ("text", "Confirm your identity"),
    ("text", "Slide to verify"),
]

# --- Upload / publish -----------------------------------------------------

# Try the legacy /upload first; TikTok redirects to creator-center.
UPLOAD_PAGE_URLS = [
    "https://www.tiktok.com/upload",
    "https://www.tiktok.com/creator-center/upload?enter_from=creator_center",
]

# File input is usually hidden (display:none) -> resolve with state="attached".
UPLOAD_FILE_INPUT = [
    ("css", "input[type='file'][accept*='video']"),
    ("css", "input[type='file']"),
    ("xpath", "//input[@type='file']"),
]

# Upload-complete + caption editor appears once the video is processed.
CAPTION_EDITOR = [
    ("css", "[contenteditable='true'][data-e2e='post-caption-text-area']"),
    ("css", "[contenteditable='true']"),
    ("role", "textbox"),
]

# Privacy ("Who can watch this video") dropdown — only if --privacy given.
PRIVACY_DROPDOWN = [
    ("css", "[data-e2e='who-can-watch-select']"),
    ("xpath", "//div[contains(@data-e2e,'who-can-watch')]"),
    ("text", "Everyone"),
]
PRIVACY_OPTIONS = ["Everyone", "Followers", "Friends"]

POST_BUTTON = [
    ("css", "button[data-e2e='post-button']"),
    ("xpath", "//button[contains(@data-e2e,'post-button') and not(@disabled)]"),
    ("xpath", "//button[normalize-space()='Post']"),
]

# Success: page navigates to /@<handle>/video/<id> or a toast appears.
POST_SUCCESS_URL_FRAGMENT = "/video/"
POST_SUCCESS_MARKERS = [
    ("text", "Your video has been posted"),
    ("text", "Posted"),
]

# --- Comments ------------------------------------------------------------

COMMENTS_CONTAINER = [
    ("css", "[data-e2e='comment-list']"),
    ("css", "div[data-e2e='comment-list-...']"),
    ("xpath", "//div[contains(@data-e2e,'comment-list')]"),
]

COMMENT_ITEM = [
    ("xpath", ".//div[contains(@data-e2e,'comment-item')]"),
    ("css", "[data-e2e^='comment-item']"),
]

COMMENT_AUTHOR = [
    ("xpath", ".//p[contains(@data-e2e,'comment-userinfo')]//a"),
    ("xpath", ".//*[contains(@data-e2e,'comment-username')]"),
    ("xpath", ".//a[contains(@href,'/@')]"),
]

COMMENT_TEXT = [
    ("xpath", ".//p[contains(@class,'comment-content') or contains(@data-e2e,'comment-content')]"),
    ("xpath", ".//*[contains(@data-e2e,'comment-desc')]"),
]

COMMENT_LIKES = [
    ("xpath", ".//*[contains(@data-e2e,'comment-like-count')]"),
    ("xpath", ".//span[contains(@data-e2e,'like-count')]"),
]

# The reply affordance on a comment.
REPLY_BUTTON = [
    ("xpath", ".//*[contains(@data-e2e,'comment-reply')]"),
    ("xpath", ".//button[normalize-space()='Reply']"),
    ("text", "Reply"),
]

# The reply input that appears after clicking Reply (inline composer).
REPLY_INPUT = [
    ("css", "[contenteditable='true'][data-e2e='comment-input']"),
    ("css", "[contenteditable='true']"),
    ("role", "textbox"),
]

REPLY_SUBMIT = [
    ("css", "[data-e2e='comment-send']"),
    ("xpath", "//button[contains(@data-e2e,'comment-send') or contains(@data-e2e,'reply-send')]"),
    ("xpath", "//button[normalize-space()='Post' or normalize-space()='Send']"),
]


def normalize_video_url(url: str) -> str:
    """Best-effort: accept a full TikTok video URL; reject bare ids in v1."""
    u = url.strip()
    if u.startswith("http"):
        return u
    # bare numeric id — can't resolve to a full URL without the handle.
    raise ValueError(
        "v1 requires the full video URL (e.g. https://www.tiktok.com/@handle/video/123). "
        "A bare id can't be resolved without the handle."
    )