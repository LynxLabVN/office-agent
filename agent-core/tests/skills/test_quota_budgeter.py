"""Unit tests for the shared quota budgeter (Phase 5.2)."""

from __future__ import annotations

from datetime import datetime, timezone

import pytest

from skills.quota_budgeter import (
    PLATFORM_CONFIG,
    QuotaBudgeter,
    QuotaExceeded,
    backoff_with_jitter,
)


def test_seventh_youtube_upload_is_blocked(tmp_path):
    """7 uploads x 1600 units = 11200 > 10000 daily limit → 7th rejected."""
    state_file = tmp_path / "quota_state.json"
    budgeter = QuotaBudgeter(state_path=state_file)
    cost = PLATFORM_CONFIG["youtube"]["cost_per_upload"]

    accepted = 0
    blocked: QuotaExceeded | None = None
    for i in range(1, 8):
        try:
            budgeter.check_budget("youtube", cost)
            budgeter.record_usage("youtube", cost)
            accepted += 1
        except QuotaExceeded as exc:
            blocked = exc

    # 6 uploads fit (6 * 1600 = 9600 <= 10000); the 7th would push to 11200.
    assert accepted == 6
    assert blocked is not None
    assert blocked.platform == "youtube"
    # retry_after must be a future, parseable ISO timestamp.
    retry_after = datetime.fromisoformat(blocked.retry_after.isoformat())
    assert retry_after > datetime.now(timezone.utc)
    assert blocked.to_dict()["error"] == "quota_exceeded"


def test_backoff_sequence_within_jitter_bounds():
    """Exponential backoff with full jitter: attempt N → [0, 2^N) seconds."""
    for attempt in range(5):
        delay = backoff_with_jitter(attempt, base_seconds=1.0, cap_seconds=60.0)
        upper = min(2**attempt, 60)
        assert 0.0 <= delay <= upper, f"attempt {attempt}: {delay} not in [0, {upper}]"


def test_backoff_caps_at_ceiling():
    """Backoff never exceeds the configured cap."""
    for attempt in range(10):
        delay = backoff_with_jitter(attempt, base_seconds=1.0, cap_seconds=8.0)
        assert delay <= 8.0


def test_record_usage_persists_state(tmp_path):
    state_file = tmp_path / "quota_state.json"
    budgeter = QuotaBudgeter(state_path=state_file)
    budgeter.record_usage("youtube", 1600)
    assert state_file.exists()
    # A fresh budgeter reads the persisted counter.
    budgeter2 = QuotaBudgeter(state_path=state_file)
    assert budgeter2.get_state()["youtube"]["used"] == 1600.0


def test_rate_header_platform_passes_budget_check(tmp_path):
    """Meta/TikTok use rate-header windows; budget check always succeeds."""
    budgeter = QuotaBudgeter(state_path=tmp_path / "quota_state.json")
    status = budgeter.check_budget("meta", cost=1.0)
    assert status["ok"] is True
    assert status["window"] == "rate_headers"