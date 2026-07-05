"""
Shared helpers for domain-specific CLI modules (marketing, hr).

- Profile-aware state persistence under ~/.hermes/<domain>/
- Best-effort MCP tool calling with graceful fallback when servers are missing
- Manager-review Telegram notification helper
- Static cron-job sync from agent-core/cron/jobs.toml

This module is intentionally thin and lazy-imports heavy Hermes machinery so
it does not slow down ``hermes --help``.
"""

from __future__ import annotations

import json
import logging
import os
import shutil
import sys
import uuid
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Dict, List, Optional

from hermes_constants import get_hermes_home
from hermes_cli.colors import Colors, color

logger = logging.getLogger(__name__)


# ---------------------------------------------------------------------------
# Paths
# ---------------------------------------------------------------------------


def _domain_dir(domain: str) -> Path:
    """Return the state directory for a domain."""
    return get_hermes_home() / domain


def _state_file(domain: str, name: str) -> Path:
    """Return the path to a domain state JSON file."""
    d = _domain_dir(domain)
    d.mkdir(parents=True, exist_ok=True)
    return d / name


def _load_json(path: Path, default: Any) -> Any:
    if path.exists():
        try:
            with open(path, "r", encoding="utf-8") as f:
                return json.load(f)
        except Exception:
            pass
    return default


def _save_json(path: Path, data: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    tmp = path.with_suffix(path.suffix + ".tmp")
    with open(tmp, "w", encoding="utf-8") as f:
        json.dump(data, f, ensure_ascii=False, indent=2)
    tmp.replace(path)


def _utcnow() -> str:
    return datetime.now(timezone.utc).isoformat()


def _new_id(prefix: str = "") -> str:
    return f"{prefix}{uuid.uuid4().hex[:12]}"


# ---------------------------------------------------------------------------
# State helpers
# ---------------------------------------------------------------------------


def load_domain_state(domain: str, filename: str, default: Optional[Dict[str, Any]] = None) -> Dict[str, Any]:
    if default is None:
        default = {"version": 1, "items": []}
    return _load_json(_state_file(domain, filename), default)


def save_domain_state(domain: str, filename: str, data: Dict[str, Any]) -> None:
    _save_json(_state_file(domain, filename), data)


def find_item(items: List[Dict[str, Any]], item_id: str) -> Optional[Dict[str, Any]]:
    for item in items:
        if item.get("id") == item_id:
            return item
    return None


def add_history(item: Dict[str, Any], state: str, note: str = "") -> None:
    item.setdefault("history", []).append(
        {"state": state, "timestamp": _utcnow(), "note": note}
    )
    item["updated_at"] = _utcnow()


# ---------------------------------------------------------------------------
# MCP call helper (best effort + fallback)
# ---------------------------------------------------------------------------


def _load_user_config() -> Dict[str, Any]:
    """Load ~/.hermes/config.yaml best-effort."""
    cfg_path = get_hermes_home() / "config.yaml"
    if not cfg_path.exists():
        return {}
    try:
        import yaml

        with open(cfg_path, "r", encoding="utf-8") as f:
            return yaml.safe_load(f) or {}
    except Exception as exc:
        logger.debug("Could not load config.yaml: %s", exc)
        return {}


def _resolve_mcp_server(alias: str) -> tuple[Optional[str], Optional[Dict[str, Any]]]:
    """Find a configured MCP server by alias or variant names."""
    cfg = _load_user_config()
    servers = cfg.get("mcp_servers") or {}
    aliases = {
        alias,
        alias.replace("mcp-", ""),
        f"mcp-{alias}",
        alias.replace("_", "-"),
        alias.replace("-", "_"),
    }
    for name, server_cfg in servers.items():
        if name in aliases:
            return name, server_cfg
    # Fallback: prefix/suffix substring match
    for name, server_cfg in servers.items():
        normalized = name.replace("mcp-", "").replace("_", "-")
        if normalized in {a.replace("mcp-", "").replace("_", "-") for a in aliases}:
            return name, server_cfg
    return None, None


def _command_available(server_cfg: Dict[str, Any]) -> bool:
    """Return True if the configured stdio command is reachable."""
    cmd = server_cfg.get("command")
    if not cmd:
        return bool(server_cfg.get("url"))
    if shutil.which(cmd):
        return True
    if Path(cmd).is_file():
        return True
    return False


def call_mcp_tool(alias: str, tool: str, args: Dict[str, Any], fallback: Optional[Any] = None) -> Any:
    """
    Call an MCP tool by server alias and tool name.

    If the server is not configured, the binary is missing, or the call fails,
    returns ``fallback`` (or ``{"error": "..."}`` if no fallback is supplied).
    """
    name, cfg = _resolve_mcp_server(alias)
    if not cfg:
        if fallback is not None:
            logger.debug("MCP server %r not configured; using fallback", alias)
            return fallback
        return {"error": f"MCP server '{alias}' is not configured"}

    if not _command_available(cfg):
        if fallback is not None:
            logger.debug("MCP server %r command not available; using fallback", name)
            return fallback
        return {"error": f"MCP server '{name}' command is not available on PATH"}

    try:
        from tools.mcp_tool import register_mcp_servers, sanitize_mcp_name_component
        from tools.registry import registry

        # Register only this server (idempotent)
        register_mcp_servers({name: cfg})

        safe_server = sanitize_mcp_name_component(name)
        safe_tool = sanitize_mcp_name_component(tool)
        tool_name = f"mcp_{safe_server}_{safe_tool}"

        result_json = registry.dispatch(tool_name, args)
        result = json.loads(result_json)
        try:
            from skills.audit import log_mcp_call

            log_mcp_call(
                actor="agent",
                server=name,
                tool=tool,
                args=args,
                result=result,
            )
        except Exception:
            pass
        if fallback is not None and isinstance(result, dict) and "error" in result:
            logger.debug("MCP call %s.%s returned error; using fallback", name, tool)
            return fallback
        return result
    except Exception as exc:
        logger.debug("MCP call %s.%s failed: %s", name, tool, exc)
        try:
            from skills.audit import log_mcp_call

            log_mcp_call(
                actor="agent",
                server=name or alias,
                tool=tool,
                args=args,
                result={"error": str(exc)},
            )
        except Exception:
            pass
        if fallback is not None:
            return fallback
        return {"error": f"MCP call to {name}.{tool} failed: {exc}"}


# ---------------------------------------------------------------------------
# Manager-review Telegram notification
# ---------------------------------------------------------------------------


def send_manager_review(
    *,
    title: str,
    summary: str,
    approve_callback: str,
    revise_callback: str,
    domain: str,
    item_id: str,
) -> Optional[str]:
    """
    Send a manager-review request to Telegram.

    Returns the tool result string, or None if MANAGER_TELEGRAM_CHAT_ID is not
    configured.
    """
    chat_id = os.getenv("MANAGER_TELEGRAM_CHAT_ID")
    if not chat_id:
        return None

    target = f"telegram:{chat_id}"
    message = (
        f"📋 *{title}*\n\n"
        f"{summary}\n\n"
        f"*Actions:*\n"
        f"✅ Approve: `{approve_callback}`\n"
        f"📝 Revise: `{revise_callback}`"
    )

    try:
        from tools.send_message_tool import send_message_tool

        result = send_message_tool(
            {
                "action": "send",
                "target": target,
                "message": message,
            }
        )
        return result
    except Exception as exc:
        logger.warning("Failed to send manager review for %s %s: %s", domain, item_id, exc)
        return None


# ---------------------------------------------------------------------------
# Static cron-job sync from agent-core/cron/jobs.toml
# ---------------------------------------------------------------------------


def _repo_root() -> Path:
    """Return the repository root (parent of hermes_cli/)."""
    return Path(__file__).resolve().parent.parent


def _static_jobs_toml() -> Optional[Path]:
    path = _repo_root() / "cron" / "jobs.toml"
    return path if path.is_file() else None


def _scripts_dir() -> Path:
    d = get_hermes_home() / "scripts"
    d.mkdir(parents=True, exist_ok=True)
    return d


def _write_shell_script(name: str, command: str) -> str:
    """Write a no-agent cron shell script under ~/.hermes/scripts/ and return its path."""
    safe_name = "".join(c if c.isalnum() or c in "-_" else "_" for c in name)
    script_path = _scripts_dir() / f"{safe_name}.sh"

    # Resolve the hermes binary for this installation so cron jobs work even
    # when the venv bin directory is not on the default PATH.
    hermes_bin = shutil.which("hermes") or str(Path(sys.executable).parent / "hermes")
    if command.strip().startswith("hermes "):
        command = f"\"{hermes_bin}\" {command.strip()[len('hermes '):]}"

    content = f"#!/usr/bin/env bash\nset -euo pipefail\n{command}\n"
    with open(script_path, "w", encoding="utf-8") as f:
        f.write(content)
    script_path.chmod(0o700)
    return str(script_path)


def sync_static_cron_jobs() -> List[str]:
    """
    Read agent-core/cron/jobs.toml and ensure each static job exists in the
    Hermes cron store (cron/jobs.json).

    Returns a list of job IDs that were created or already existed.
    """
    toml_path = _static_jobs_toml()
    if not toml_path:
        return []

    try:
        if sys.version_info >= (3, 11):
            import tomllib

            with open(toml_path, "rb") as f:
                data = tomllib.load(f)
        else:
            import tomli as tomllib

            with open(toml_path, "rb") as f:
                data = tomllib.load(f)
    except Exception as exc:
        logger.debug("Could not parse jobs.toml: %s", exc)
        return []

    jobs = data.get("jobs", [])
    if not jobs:
        return []

    try:
        from cron.jobs import create_job, load_jobs
    except Exception as exc:
        logger.debug("Could not import cron.jobs: %s", exc)
        return []

    existing_names = {j.get("name") for j in load_jobs()}
    created: List[str] = []

    for job in jobs:
        name = job.get("name")
        schedule = job.get("schedule")
        command = job.get("command")
        if not name or not schedule or not command:
            continue
        if name in existing_names:
            created.append(name)
            continue
        try:
            script_path = _write_shell_script(name, command)
            created_job = create_job(
                prompt=None,
                schedule=schedule,
                name=name,
                script=script_path,
                no_agent=True,
                deliver="local",
            )
            created.append(created_job.get("id") or name)
            existing_names.add(name)
        except Exception as exc:
            logger.debug("Failed to create static cron job %r: %s", name, exc)

    return created


def print_mcp_fallback_notice(domain: str, alias: str, tool: str) -> None:
    """Print a dim notice that an MCP call was not available and fallback was used."""
    print(
        color(
            f"  (MCP server '{alias}' tool '{tool}' not available — using demo fallback)",
            Colors.DIM,
        )
    )
