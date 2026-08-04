"""Standalone browser-UI automation tool for publishing videos and replying to
comments on TikTok (v1) and Facebook (v2) as a logged-in normal account.

Invoked via ``python -m social`` from ``agent-core/``. Deliberately standalone:
imports NO ``hermes_cli`` / ``hermes_constants`` / ``colors`` or any module deleted
by the 6a6383f7d refactor. Only third-party deps: ``playwright`` (install via
``uv pip install playwright``) and the already-installed FastAPI stack.

Risk: browser-UI automation of FB/TikTok as a normal account violates their
Terms of Service and risks shadow-ban/account-lock. Mitigations are intentionally
minimal (real persistent Chrome profile, human-like typing, headed by default).
No stealth plugins. Low-volume personal use only.
"""

__version__ = "0.1.0"