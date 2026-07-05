"""Quota budgeter implementation.

Tracks per-platform daily/minute usage and computes retry-after timestamps
plus exponential backoff with jitter.
"""

from __future__ import annotations

import json
import random
import time
from dataclasses import dataclass, field
from datetime import datetime, time as dt_time, timedelta, timezone
from pathlib import Path
from typing import Any, Dict, Optional

from hermes_constants import get_hermes_home

DEFAULT_STATE_FILE = "quota_state.json"

PLATFORM_CONFIG: Dict[str, Dict[str, Any]] = {
    "youtube": {
        "daily_limit": 10_000,
        "cost_per_upload": 1_600,
        "window": "daily",
        "reset_timezone": "America/Los_Angeles",
    },
    "meta": {
        "daily_limit": None,
        "window": "rate_headers",
        "header_remaining": "x-business-use-case-usage",
    },
    "tiktok": {
        "daily_limit": None,
        "window": "rate_headers",
        "header_remaining": "x-ratelimit-remaining",
    },
    "calcom": {
        "per_minute_limit": 120,
        "window": "per_minute",
    },
}


class QuotaExceeded(Exception):
    """Raised when a platform quota would be exceeded."""

    def __init__(self, platform: str, retry_after: datetime, message: str = ""):
        self.platform = platform
        self.retry_after = retry_after
        self.message = message or f"{platform} quota exceeded; retry after {retry_after.isoformat()}"
        super().__init__(self.message)

    def to_dict(self) -> Dict[str, Any]:
        return {
            "error": "quota_exceeded",
            "platform": self.platform,
            "retry_after": self.retry_after.isoformat(),
        }


@dataclass
class PlatformState:
    used: float = 0.0
    reset_at: Optional[str] = None
    extra: Dict[str, Any] = field(default_factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        return {
            "used": self.used,
            "reset_at": self.reset_at,
            "extra": self.extra,
        }

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> "PlatformState":
        return cls(
            used=float(data.get("used", 0.0)),
            reset_at=data.get("reset_at"),
            extra=data.get("extra", {}) or {},
        )


class QuotaBudgeter:
    """Centralized quota tracker with persistent JSON state."""

    def __init__(self, state_path: Optional[Path] = None):
        self.state_path = state_path or (
            get_hermes_home() / "data" / DEFAULT_STATE_FILE
        )
        self._state: Dict[str, PlatformState] = {}
        self._load()

    def _load(self) -> None:
        if self.state_path.exists():
            try:
                raw = json.loads(self.state_path.read_text(encoding="utf-8"))
                self._state = {
                    k: PlatformState.from_dict(v) for k, v in raw.get("platforms", {}).items()
                }
            except Exception:
                self._state = {}
        else:
            self._state = {}

    def _save(self) -> None:
        self.state_path.parent.mkdir(parents=True, exist_ok=True)
        payload = {
            "updated_at": datetime.now(timezone.utc).isoformat(),
            "platforms": {k: v.to_dict() for k, v in self._state.items()},
        }
        tmp = self.state_path.with_suffix(".tmp")
        tmp.write_text(json.dumps(payload, ensure_ascii=False, indent=2), encoding="utf-8")
        tmp.replace(self.state_path)

    def _platform_state(self, platform: str) -> PlatformState:
        if platform not in self._state:
            self._state[platform] = PlatformState()
        return self._state[platform]

    def _is_past_reset(self, reset_at: Optional[str]) -> bool:
        if not reset_at:
            return True
        try:
            dt = datetime.fromisoformat(reset_at)
            return datetime.now(timezone.utc) >= dt
        except Exception:
            return True

    def _next_reset(self, platform: str) -> datetime:
        cfg = PLATFORM_CONFIG.get(platform, {})
        tz_name = cfg.get("reset_timezone", "UTC")
        if tz_name == "America/Los_Angeles":
            # Midnight Pacific Time.
            import zoneinfo  # available in Python 3.9+

            tz = zoneinfo.ZoneInfo("America/Los_Angeles")
            now = datetime.now(tz)
            midnight = datetime.combine(now.date() + timedelta(days=1), dt_time.min, tz)
            return midnight.astimezone(timezone.utc)
        # Default: next UTC midnight.
        now = datetime.now(timezone.utc)
        return datetime.combine(now.date() + timedelta(days=1), dt_time.min, timezone.utc)

    def check_budget(self, platform: str, cost: float = 1.0) -> Dict[str, Any]:
        """Return budget status; raise QuotaExceeded if the call would exceed it."""
        cfg = PLATFORM_CONFIG.get(platform, {})
        state = self._platform_state(platform)

        if self._is_past_reset(state.reset_at):
            state.used = 0.0
            state.reset_at = self._next_reset(platform).isoformat()

        limit: Optional[float] = cfg.get("daily_limit") or cfg.get("per_minute_limit")
        if limit is not None:
            remaining = limit - state.used
            if remaining < cost:
                retry_after = datetime.fromisoformat(state.reset_at)
                raise QuotaExceeded(platform, retry_after)
            return {
                "ok": True,
                "platform": platform,
                "used": state.used,
                "limit": limit,
                "remaining": remaining - cost,
            }

        # Rate-header windows always succeed the budget check; callers should
        # update with header values after the request.
        return {"ok": True, "platform": platform, "window": "rate_headers"}

    def record_usage(self, platform: str, cost: float = 1.0) -> Dict[str, Any]:
        """Increment usage after a successful call."""
        state = self._platform_state(platform)
        if self._is_past_reset(state.reset_at):
            state.used = 0.0
            state.reset_at = self._next_reset(platform).isoformat()
        state.used += cost
        self._save()
        return {"ok": True, "platform": platform, "used": state.used}

    def update_from_headers(self, platform: str, headers: Dict[str, str]) -> None:
        """Update state from platform rate-limit headers (Meta/TikTok)."""
        cfg = PLATFORM_CONFIG.get(platform, {})
        header = cfg.get("header_remaining", "x-ratelimit-remaining")
        value = headers.get(header.lower()) or headers.get(header)
        if value is not None:
            state = self._platform_state(platform)
            try:
                state.extra["remaining_from_header"] = float(value)
            except ValueError:
                pass
        self._save()

    def get_state(self) -> Dict[str, Dict[str, Any]]:
        return {k: v.to_dict() for k, v in self._state.items()}


def backoff_with_jitter(
    attempt: int,
    base_seconds: float = 1.0,
    cap_seconds: float = 60.0,
) -> float:
    """Return exponential backoff delay with full jitter.

    Attempt 0 -> [0, base), attempt 1 -> [0, base*2), attempt 2 -> [0, base*4).
    """
    if attempt < 0:
        attempt = 0
    delay = min(base_seconds * (2**attempt), cap_seconds)
    return random.uniform(0.0, delay)


_budgeter_singleton: Optional[QuotaBudgeter] = None


def get_budgeter(state_path: Optional[Path] = None) -> QuotaBudgeter:
    """Return the shared QuotaBudgeter instance."""
    global _budgeter_singleton
    if _budgeter_singleton is None or state_path is not None:
        return QuotaBudgeter(state_path)
    return _budgeter_singleton


if __name__ == "__main__":
    import sys

    # Self-test: 7 YouTube uploads should block the 7th.
    from pathlib import Path

    tmp = Path(__file__).with_suffix(".test_state.json")
    if tmp.exists():
        tmp.unlink()
    budgeter = QuotaBudgeter(tmp)
    upload_cost = PLATFORM_CONFIG["youtube"]["cost_per_upload"]
    for i in range(1, 8):
        try:
            budgeter.check_budget("youtube", upload_cost)
            budgeter.record_usage("youtube", upload_cost)
            print(f"upload {i}: OK")
        except QuotaExceeded as exc:
            print(f"upload {i}: BLOCKED retry_after={exc.retry_after.isoformat()}")
            if i != 7:
                sys.exit(1)
            break
    else:
        print("ERROR: expected the 7th upload to be blocked")
        sys.exit(1)

    # Backoff sequence check.
    for attempt in range(4):
        delay = backoff_with_jitter(attempt, base_seconds=1.0, cap_seconds=60.0)
        print(f"backoff attempt {attempt}: {delay:.3f}s")
        upper = min(2**attempt, 60)
        assert 0 <= delay <= upper, f"delay {delay} out of bounds for attempt {attempt}"
    print("quota-budgeter self-test passed")
    tmp.unlink(missing_ok=True)
