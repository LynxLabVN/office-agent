"""Audit logger implementation.

Writes JSONL lines with the schema:

    {
        "ts": "2026-07-06T00:00:00+00:00",
        "actor": "agent" | "human:<name>",
        "action": "state_transition | mcp_call | auto_reply | human_decision | ...",
        "target": "piece-id | candidate-id | ...",
        "before": { ... },
        "after": { ... },
        "meta": { ... }
    }
"""

from __future__ import annotations

import json
import threading
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Dict, Optional

from hermes_constants import get_hermes_home

DEFAULT_AUDIT_FILE = "audit.log"


class AuditLogger:
    """Thread-safe JSONL audit logger."""

    def __init__(self, log_path: Optional[Path] = None):
        self.log_path = log_path or (get_hermes_home() / "data" / DEFAULT_AUDIT_FILE)
        self._lock = threading.Lock()

    def _append(self, entry: Dict[str, Any]) -> None:
        self.log_path.parent.mkdir(parents=True, exist_ok=True)
        line = json.dumps(entry, ensure_ascii=False, sort_keys=True)
        with self._lock:
            with open(self.log_path, "a", encoding="utf-8") as f:
                f.write(line + "\n")

    def log(
        self,
        *,
        actor: str,
        action: str,
        target: str,
        before: Optional[Dict[str, Any]] = None,
        after: Optional[Dict[str, Any]] = None,
        meta: Optional[Dict[str, Any]] = None,
    ) -> Dict[str, Any]:
        entry = {
            "ts": datetime.now(timezone.utc).isoformat(),
            "actor": actor,
            "action": action,
            "target": target,
        }
        if before is not None:
            entry["before"] = before
        if after is not None:
            entry["after"] = after
        if meta is not None:
            entry["meta"] = meta
        self._append(entry)
        return entry

    def query(
        self,
        from_ts: Optional[str] = None,
        to_ts: Optional[str] = None,
        actor: Optional[str] = None,
        action: Optional[str] = None,
        target: Optional[str] = None,
        limit: int = 10_000,
    ) -> list[Dict[str, Any]]:
        results: list[Dict[str, Any]] = []
        if not self.log_path.exists():
            return results

        with open(self.log_path, "r", encoding="utf-8") as f:
            for line in f:
                line = line.strip()
                if not line:
                    continue
                try:
                    entry = json.loads(line)
                except json.JSONDecodeError:
                    continue
                if from_ts and entry.get("ts", "") < from_ts:
                    continue
                if to_ts and entry.get("ts", "") > to_ts:
                    continue
                if actor and entry.get("actor") != actor:
                    continue
                if action and entry.get("action") != action:
                    continue
                if target and entry.get("target") != target:
                    continue
                results.append(entry)
                if len(results) >= limit:
                    break
        return results


_logger_singleton: Optional[AuditLogger] = None
_singleton_lock = threading.Lock()


def get_logger(log_path: Optional[Path] = None) -> AuditLogger:
    """Return the shared audit logger."""
    global _logger_singleton
    if _logger_singleton is None or log_path is not None:
        with _singleton_lock:
            if _logger_singleton is None or log_path is not None:
                _logger_singleton = AuditLogger(log_path)
    return _logger_singleton


def log_state_transition(
    actor: str,
    target: str,
    before: Dict[str, Any],
    after: Dict[str, Any],
    meta: Optional[Dict[str, Any]] = None,
) -> Dict[str, Any]:
    return get_logger().log(
        actor=actor,
        action="state_transition",
        target=target,
        before=before,
        after=after,
        meta=meta,
    )


def log_mcp_call(
    actor: str,
    server: str,
    tool: str,
    args: Dict[str, Any],
    result: Any,
    meta: Optional[Dict[str, Any]] = None,
) -> Dict[str, Any]:
    return get_logger().log(
        actor=actor,
        action="mcp_call",
        target=f"{server}.{tool}",
        before={"args": args},
        after={"result": result},
        meta=meta,
    )


def log_auto_reply(
    actor: str,
    target: str,
    message: str,
    meta: Optional[Dict[str, Any]] = None,
) -> Dict[str, Any]:
    return get_logger().log(
        actor=actor,
        action="auto_reply",
        target=target,
        after={"message": message},
        meta=meta,
    )


def log_decision(
    actor: str,
    target: str,
    decision: str,
    context: Optional[Dict[str, Any]] = None,
) -> Dict[str, Any]:
    return get_logger().log(
        actor=actor,
        action="human_decision" if actor.startswith("human:") else "decision",
        target=target,
        after={"decision": decision, "context": context or {}},
    )


def log_pii_access(
    actor: str,
    target: str,
    data_type: str,
    meta: Optional[Dict[str, Any]] = None,
) -> Dict[str, Any]:
    return get_logger().log(
        actor=actor,
        action="pii_access",
        target=target,
        after={"data_type": data_type},
        meta=meta,
    )
