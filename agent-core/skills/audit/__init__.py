"""Shared audit logger.

Appends JSONL entries for every state transition, MCP tool call, auto-reply,
and human decision to ``~/.hermes/data/audit.log``.
"""

from __future__ import annotations

from .logger import (
    AuditLogger,
    log_decision,
    log_mcp_call,
    log_pii_access,
    log_state_transition,
)

__all__ = [
    "AuditLogger",
    "log_decision",
    "log_mcp_call",
    "log_pii_access",
    "log_state_transition",
]
