"""Role-based access helpers for HR dashboard endpoints."""

from __future__ import annotations

from typing import Any

from fastapi import HTTPException, Request


def _load_role_from_config() -> str:
    """Load the local user role from Hermes config.yaml."""
    try:
        from hermes_cli.config import load_config

        cfg = load_config()
        return str(cfg.get("user", {}).get("role", "")).lower()
    except Exception:
        return ""


def _get_user_role(request: Request) -> str:
    """Best-effort role resolution from request state, headers, or config."""
    # If dashboard_auth populated request.state.user, use it.
    user: Any = getattr(request.state, "user", None)
    if isinstance(user, dict):
        return str(user.get("role", "")).lower()
    # Test / programmatic override via header.
    role_header = request.headers.get("x-recruiter-role", "")
    if role_header:
        return role_header.lower()
    return _load_role_from_config()


def require_recruiter(request: Request) -> str:
    """Raise 403 unless the current user has the recruiter role."""
    role = _get_user_role(request)
    if role != "recruiter":
        raise HTTPException(status_code=403, detail="Recruiter role required")
    return role


def is_recruiter(request: Request) -> bool:
    return _get_user_role(request) == "recruiter"
