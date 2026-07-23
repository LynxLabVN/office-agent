#!/usr/bin/env python3
"""
Bundled MCP Sync -- first-launch seeding of optional-mcps into ~/.hermes/config.yaml.

Mirrors tools/skills_sync.py (bundled-skills seeding) for MCP servers. On the
self-contained desktop build there is no `hermes mcp install` clone step -- the
app ships with the optional-mcps either as remote-HTTP URLs (linear,
unreal-engine: just config entries) or as a bundled executable (n8n:
resources/mcps/<name>.exe). This module writes those entries into
~/.hermes/config.yaml under `mcp_servers` on first launch, idempotently, so a
fresh install sees the bundled MCPs registered without a manual `hermes mcp add`.

Design notes:
  - Idempotent: an entry already present in `mcp_servers` is NEVER overwritten --
    user edits/`hermes mcp configure` are respected.
  - Opt-out: `~/.hermes/.no-bundled-mcps` marker skips seeding entirely (mirrors
    skills_sync's `.no-bundled-skills`).
  - stdio entries with a bundled exe: command = absolute path into the app
    resources (resolved via HERMES_DESKTOP_RESOURCES env, set by the Electron
    main process). tools/mcp_tool._resolve_stdio_command uses a command
    containing a path separator as-is -- no PATH lookup -- so the absolute path
    spawns the bundled binary directly.
  - api_key auth (n8n): registered with enabled=false because the user must
    supply API-key env vars first (`hermes mcp configure n8n`). oauth/none
    (linear/unreal): enabled=true; OAuth runs on first connect, unreal is a
    no-op until the editor server is up.
  - Falls back to a hardcoded default set for the 3 known optional-mcps when
    the manifest catalog is unavailable (e.g. manifests not bundled with the
    PyInstaller exe), so seeding still works.
"""

import logging
import os
from pathlib import Path
from typing import Any, Dict, List, Optional

from hermes_constants import get_hermes_home

logger = logging.getLogger(__name__)

HERMES_HOME = get_hermes_home()
NO_BUNDLED_MCPS_MARKER = ".no-bundled-mcps"

# Bundled stdio MCP executable search names under <resources>/mcps/.
# <name>.exe is preferred; <name>-mcp.exe is the fallback convention.
_BUNDLED_EXE_CANDIDATES = {
    "n8n": ("n8n.exe", "n8n-mcp.exe"),
}


def _marker_path() -> Path:
    return HERMES_HOME / NO_BUNDLED_MCPS_MARKER


def _resources_mcps_dir() -> Optional[Path]:
    """Return the bundled mcps/ directory if running inside the packaged app."""
    resources = os.getenv("HERMES_DESKTOP_RESOURCES", "").strip()
    if not resources:
        return None
    p = Path(resources) / "mcps"
    return p if p.is_dir() else None


def _find_bundled_mcp_exe(name: str) -> Optional[Path]:
    """Locate a bundled stdio MCP executable for *name* in resources/mcps/."""
    mcps_dir = _resources_mcps_dir()
    if mcps_dir is None:
        return None
    candidates = _BUNDLED_EXE_CANDIDATES.get(name, (f"{name}.exe", f"{name}-mcp.exe"))
    for cand in candidates:
        exe = mcps_dir / cand
        if exe.exists():
            return exe
    return None


def _installed_servers() -> Dict[str, dict]:
    """Return the existing `mcp_servers` dict from config.yaml (empty if none)."""
    try:
        from hermes_cli.mcp_config import _get_mcp_servers
        return _get_mcp_servers()
    except Exception:
        logger.debug("bundled-mcp: could not load existing mcp_servers", exc_info=True)
        return {}


def _save_server(name: str, server_config: dict) -> bool:
    try:
        from hermes_cli.mcp_config import _save_mcp_server
        return _save_mcp_server(name, server_config)
    except Exception:
        logger.debug("bundled-mcp: save failed for %s", name, exc_info=True)
        return False


def _entries_from_catalog() -> List[Any]:
    """Parse optional-mcps manifests via the catalog. Empty list on any failure."""
    try:
        from hermes_cli.mcp_catalog import list_catalog
        entries = list_catalog()
        return entries or []
    except Exception:
        logger.debug("bundled-mcp: catalog unavailable, using hardcoded fallback", exc_info=True)
        return []


def _hardcoded_defaults() -> List[Dict[str, Any]]:
    """Fallback server-config blocks for the 3 known optional-mcps.

    Used only when the manifest catalog is unavailable. Keeps the seeding
    functional in a PyInstaller bundle that didn't ship the manifest yamls.
    """
    return [
        {
            "name": "linear",
            "transport": {"type": "http", "url": "https://mcp.linear.app/mcp"},
            "auth": {"type": "oauth"},
        },
        {
            "name": "unreal-engine",
            "transport": {"type": "http", "url": "http://127.0.0.1:8000/mcp"},
            "auth": {"type": "none"},
        },
        {
            "name": "n8n",
            "transport": {"type": "stdio"},
            "auth": {"type": "api_key"},
        },
    ]


def _config_for_http(url: str, auth_type: str) -> dict:
    cfg: Dict[str, Any] = {"url": url, "enabled": True}
    if auth_type == "oauth":
        cfg["auth"] = "oauth"
    return cfg


def _config_for_bundled_stdio(exe: Path) -> dict:
    # Absolute path with a separator -> _resolve_stdio_command uses it as-is.
    return {"command": str(exe), "args": [], "enabled": False}


def _build_from_entry(entry: Any) -> Optional[dict]:
    """Translate a catalog entry to a config block for the bundled build.

    Returns None when the entry cannot be satisfied bundled (stdio with no
    bundled exe) and should be skipped rather than cloned.
    """
    t = entry.transport
    auth_type = entry.auth.type if entry.auth else "none"
    if t.type == "http":
        return _config_for_http(t.url, auth_type)
    if t.type == "stdio":
        exe = _find_bundled_mcp_exe(entry.name)
        if exe is None:
            return None
        cfg = _config_for_bundled_stdio(exe)
        # n8n is api_key -> needs user-supplied env vars; stay disabled until
        # `hermes mcp configure n8n` sets them.
        cfg["enabled"] = auth_type != "api_key"
        return cfg
    return None


def _build_from_default(default: Dict[str, Any]) -> Optional[dict]:
    t = default["transport"]
    auth_type = default["auth"]["type"]
    if t["type"] == "http":
        return _config_for_http(t["url"], auth_type)
    if t["type"] == "stdio":
        exe = _find_bundled_mcp_exe(default["name"])
        if exe is None:
            return None
        cfg = _config_for_bundled_stdio(exe)
        cfg["enabled"] = auth_type != "api_key"
        return cfg
    return None


def sync_bundled_mcps(quiet: bool = True) -> int:
    """Seed bundled optional-mcps into ~/.hermes/config.yaml.

    Adds an entry only when no entry with that name already exists. Returns the
    number of entries written. No-op when the opt-out marker is present or when
    not running inside the packaged app AND no manifest catalog is available.
    """
    if _marker_path().exists():
        return 0

    existing = _installed_servers()
    written = 0

    entries = _entries_from_catalog()
    if entries:
        for entry in entries:
            if entry.name in existing:
                continue
            cfg = _build_from_entry(entry)
            if cfg is None:
                if not quiet:
                    print(f"  · {entry.name}: no bundled exe, skipped (run `hermes mcp install {entry.name}` to clone)")
                continue
            if _save_server(entry.name, cfg):
                written += 1
                if not quiet:
                    print(f"  ✓ {entry.name}: registered (enabled={cfg.get('enabled')})")
        return written

    # Fallback: catalog unavailable -- use the hardcoded default set, but only
    # when we're inside the packaged app (have a resources dir) so a source-tree
    # run doesn't seed a stub config.
    if _resources_mcps_dir() is None and not os.getenv("HERMES_DESKTOP"):
        return 0
    for default in _hardcoded_defaults():
        name = default["name"]
        if name in existing:
            continue
        cfg = _build_from_default(default)
        if cfg is None:
            continue
        if _save_server(name, cfg):
            written += 1
            if not quiet:
                print(f"  ✓ {name}: registered (enabled={cfg.get('enabled')})")
    return written