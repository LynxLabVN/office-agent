"""Shared quota budgeter for external API platforms.

Provides centralized quota tracking and rate-limit backoff for YouTube,
Meta, TikTok, and Cal.com. State is persisted in
``~/.hermes/data/quota_state.json`` so multiple processes share the same
counters and reset times.
"""

from __future__ import annotations

from .budgeter import (
    PLATFORM_CONFIG,
    QuotaBudgeter,
    QuotaExceeded,
    backoff_with_jitter,
    get_budgeter,
)

__all__ = [
    "PLATFORM_CONFIG",
    "QuotaBudgeter",
    "QuotaExceeded",
    "backoff_with_jitter",
    "get_budgeter",
]
