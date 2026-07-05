"""Audit log REST API for the LynxLabVN dashboard.

Returns JSONL-compatible JSON arrays filtered by time, actor, action, and
target.
"""

from __future__ import annotations

from typing import Any, Dict, List, Optional

from fastapi import APIRouter, Query
from pydantic import BaseModel

from skills.audit import AuditLogger

router = APIRouter(prefix="/api/audit", tags=["audit"])


def _get_logger() -> AuditLogger:
    return AuditLogger()


@router.get("")
async def query_audit(
    from_ts: Optional[str] = Query(None),
    to_ts: Optional[str] = Query(None),
    actor: Optional[str] = Query(None),
    action: Optional[str] = Query(None),
    target: Optional[str] = Query(None),
    limit: int = Query(10_000, ge=1, le=50_000),
) -> Dict[str, List[Dict[str, Any]]]:
    """Query audit log entries."""
    logger = _get_logger()
    entries = logger.query(
        from_ts=from_ts,
        to_ts=to_ts,
        actor=actor,
        action=action,
        target=target,
        limit=limit,
    )
    return {"entries": entries}


class AppendAuditRequest(BaseModel):
    actor: str
    action: str
    target: str
    before: Optional[Dict[str, Any]] = None
    after: Optional[Dict[str, Any]] = None
    meta: Optional[Dict[str, Any]] = None


@router.post("")
async def append_audit(body: AppendAuditRequest) -> Dict[str, Any]:
    """Append a single audit entry (dashboard or external integrations)."""
    logger = _get_logger()
    entry = logger.log(
        actor=body.actor,
        action=body.action,
        target=body.target,
        before=body.before,
        after=body.after,
        meta=body.meta,
    )
    return {"ok": True, "entry": entry}
