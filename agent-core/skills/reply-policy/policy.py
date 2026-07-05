"""Shared reply-policy engine for social-media and HR messaging.

Provides allow-listed template matching and a lightweight, regex-based
LLM guard.  The policy decides whether to send, queue for human review,
or drop an outbound reply.
"""

from __future__ import annotations

import os
import re
from typing import Any, Optional


def _load_toml(path: str) -> dict:
    """Load a TOML file using the best available parser."""
    try:
        import tomllib  # Python >= 3.11
    except ImportError:  # pragma: no cover
        try:
            import tomli as tomllib  # type: ignore[no-redef]
        except ImportError as exc:
            raise ImportError(
                "TOML parser not available. Install Python >= 3.11 or `tomli`."
            ) from exc

    with open(path, "rb") as f:
        return tomllib.load(f)


def load_templates(path: Optional[str] = None) -> dict:
    """Load approved reply templates from a TOML file.

    Args:
        path: Path to the templates TOML file. Defaults to
            ``templates.toml`` in the same directory as this module.

    Returns:
        A nested dict mapping ``platform -> scenario -> template``.
    """
    if path is None:
        path = os.path.join(os.path.dirname(__file__), "templates.toml")
    return _load_toml(path)


def match_template(
    platform: str, scenario: str, inbound_text: str, templates: dict
) -> Optional[dict]:
    """Return the matching template for a platform/scenario pair.

    Args:
        platform: Platform key, e.g. ``youtube`` or ``zalo_oa``.
        scenario: Scenario key, e.g. ``youtube_thank_you``.
        inbound_text: Original inbound message (unused by the simple matcher
            but accepted for future keyword-based routing).
        templates: Templates dictionary from :func:`load_templates`.

    Returns:
        The template dict (with the platform/scenario keys added) or ``None``.
    """
    platform = (platform or "").lower()
    scenario = (scenario or "").lower()
    platform_templates = templates.get(platform)
    if not isinstance(platform_templates, dict):
        return None
    template = platform_templates.get(scenario)
    if not isinstance(template, dict):
        return None
    result = dict(template)
    result["platform"] = platform
    result["scenario"] = scenario
    return result


# Simple regex patterns for the lightweight guard.
_PHONE_RE = re.compile(r"\b(\+?\d[\d\s\-\.]{7,}\d)\b")
_EMAIL_RE = re.compile(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}\b")
_SALARY_RE = re.compile(
    r"(salary|lương|thu nhập|mức lương).*?(\d|usd|vnd|vnđ|triệu|million)",
    re.IGNORECASE,
)
_PROMISE_RE = re.compile(
    r"\b(we guarantee|we promise|guaranteed|cam kết|đảm bảo|chắc chắn)\b",
    re.IGNORECASE,
)
_MEDICAL_LEGAL_RE = re.compile(
    r"\b(diagnos(e|is)|treatment|cure|legal advice|luật sư|khám bệnh|chữa khỏi|\blawyer\b)\b",
    re.IGNORECASE,
)
_MAX_LENGTH = 200


def llm_guard(reply_text: str, context: dict) -> dict:
    """Lightweight rule-based guard for outbound replies.

    Args:
        reply_text: The rendered reply text to evaluate.
        context: Optional context dict. ``domain='hr'`` enables salary checks.

    Returns:
        ``{"approved": bool, "reason": str}``.
    """
    text = reply_text or ""

    checks: list[tuple[bool, str]] = [
        (len(text) > _MAX_LENGTH, f"exceeds {_MAX_LENGTH} characters"),
        (
            bool(_PROMISE_RE.search(text)),
            "contains promise/commitment language",
        ),
        (
            bool(_PHONE_RE.search(text)),
            "contains possible phone number (PII)",
        ),
        (
            bool(_EMAIL_RE.search(text)),
            "contains possible email address (PII)",
        ),
        (
            bool(_MEDICAL_LEGAL_RE.search(text)),
            "contains medical or legal claim",
        ),
        (
            context.get("domain") == "hr" and bool(_SALARY_RE.search(text)),
            "contains salary specifics in HR context",
        ),
    ]

    for failed, reason in checks:
        if failed:
            return {"approved": False, "reason": reason}

    return {"approved": True, "reason": "passed lightweight guard"}


def decide_reply(
    inbound: str,
    platform: str,
    scenario: str,
    mode: str,
    templates: dict,
    context: dict,
) -> dict[str, Any]:
    """Decide whether to send, queue, or drop an outbound reply.

    Args:
        inbound: The inbound message text.
        platform: Platform key.
        scenario: Scenario key.
        mode: One of ``auto``, ``suggest``, or ``off``.
        templates: Templates dictionary.
        context: Optional context dict passed to the guard.

    Returns:
        ``{"action": "send"|"queue_human"|"drop", "reply": str|None,
        "reason": str}``.
    """
    mode = (mode or "suggest").lower()

    if mode == "off":
        return {"action": "drop", "reply": None, "reason": "reply policy is off"}

    template = match_template(platform, scenario, inbound, templates)
    if template is None:
        return {
            "action": "drop",
            "reply": None,
            "reason": f"no template for {platform}/{scenario}",
        }

    reply_text = template.get("text", "")

    if mode == "suggest":
        return {
            "action": "queue_human",
            "reply": reply_text,
            "reason": "suggest mode requires human approval",
        }

    # auto mode: run guard
    guard = llm_guard(reply_text, context)
    if guard["approved"]:
        return {
            "action": "send",
            "reply": reply_text,
            "reason": "template matched and guard passed",
        }

    return {
        "action": "queue_human",
        "reply": reply_text,
        "reason": f"guard rejected: {guard['reason']}",
    }
