"""Unit tests for the audit logger (Phase 5.4)."""

from __future__ import annotations

import json
from pathlib import Path

import pytest

import skills.audit.logger as audit_module
from skills.audit import (
    AuditLogger,
    log_auto_reply,
    log_decision,
    log_mcp_call,
    log_pii_access,
    log_state_transition,
)
from skills.audit.logger import get_logger


@pytest.fixture
def audit_path(tmp_path, monkeypatch):
    """Point the shared audit logger singleton at a per-test log file."""
    log_path = tmp_path / "audit.log"
    # Reset the singleton so the helpers write to our temp path.
    audit_module._logger_singleton = None
    get_logger(log_path=log_path)
    yield log_path
    audit_module._logger_singleton = None


def _read_log(path: Path) -> list[dict]:
    entries = []
    for line in path.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if line:
            entries.append(json.loads(line))
    return entries


def test_full_pipeline_cycle_produces_jsonl_trail(audit_path):
    """A full marketing pipeline cycle emits one JSONL entry per transition + MCP call."""
    # State transitions through the pipeline.
    for before, after in [
        ("PRODUCT_SELECT", "FORMAT_PICK"),
        ("FORMAT_PICK", "SCRIPT_DRAFT"),
        ("SCRIPT_DRAFT", "HOOK_ITERATE"),
        ("HOOK_ITERATE", "MANAGER_REVIEW_GATE"),
    ]:
        log_state_transition(
            actor="agent",
            target="piece-1",
            before={"state": before},
            after={"state": after},
        )

    # An MCP tool call.
    log_mcp_call(
        actor="agent",
        server="mcp-video-edit",
        tool="render",
        args={"piece_id": "piece-1"},
        result={"output": "/tmp/render.mp4"},
    )

    # A human decision at the manager review gate.
    log_decision(
        actor="human:linh",
        target="piece-1",
        decision="approved",
        context={"gate": "MANAGER_REVIEW_GATE"},
    )

    # An auto-reply during monitoring.
    log_auto_reply(
        actor="agent",
        target="comment-42",
        message="Cảm ơn bạn đã quan tâm!",
    )

    entries = _read_log(audit_path)
    actions = [e["action"] for e in entries]

    # One entry per state transition + one per MCP call + decision + auto-reply.
    assert actions.count("state_transition") == 4
    assert "mcp_call" in actions
    assert "human_decision" in actions
    assert "auto_reply" in actions
    assert len(entries) == 7

    # Every entry has the required schema fields.
    for entry in entries:
        assert {"ts", "actor", "action", "target"} <= set(entry)


def test_query_filters_by_actor_and_action(audit_path):
    log_state_transition("agent", "piece-1", {"state": "a"}, {"state": "b"})
    log_decision("human:linh", "piece-1", "approved")
    log_pii_access("human:linh", "candidate-7", "cv_pdf")

    logger = AuditLogger(log_path=audit_path)
    human_entries = logger.query(actor="human:linh")
    assert len(human_entries) == 2
    assert all(e["actor"] == "human:linh" for e in human_entries)

    pii_entries = logger.query(action="pii_access")
    assert len(pii_entries) == 1
    assert pii_entries[0]["target"] == "candidate-7"


def test_query_filters_by_time_window(audit_path):
    log_state_transition("agent", "piece-1", {"state": "a"}, {"state": "b"})
    logger = AuditLogger(log_path=audit_path)
    entries = logger.query(from_ts="1970-01-01T00:00:00+00:00")
    assert len(entries) == 1
    future = logger.query(from_ts="9999-01-01T00:00:00+00:00")
    assert future == []


def test_mcp_call_records_server_and_tool(audit_path):
    log_mcp_call(
        actor="agent",
        server="mcp-ledger",
        tool="append_entry",
        args={"piece_id": "piece-1"},
        result={"ok": True},
    )
    entries = _read_log(audit_path)
    assert entries[0]["target"] == "mcp-ledger.append_entry"
    assert entries[0]["before"]["args"]["piece_id"] == "piece-1"


def test_pii_access_is_logged(audit_path):
    """get_user_profile / CV access must leave an audit trail (Phase 5.7)."""
    log_pii_access(actor="human:linh", target="candidate-7", data_type="cv_pdf")
    entries = _read_log(audit_path)
    assert entries[0]["action"] == "pii_access"
    assert entries[0]["after"]["data_type"] == "cv_pdf"