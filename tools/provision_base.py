#!/usr/bin/env python3
"""Provision the Hermes base into agent-core/ (dev) or overlay it (installer).

office-agent is a slim overlay on top of the Hermes base (fork
LynxLabVN/agent-core @ commit 88d1d6206f399c134d1f4c0b7db27733aaa3c50c).
Only the office-agent delta (16 modified + 65 added files) is tracked in git;
the 5929 byte-identical base files are listed in tools/base_files.txt and are
restored by this script so the editable venv / the installer can run.

Modes
-----
  dev (default): copy each base file from a base source into agent-core/,
                 never overwriting the tracked delta. Used after a fresh clone
                 or after `git rm` of the base files.

  install: pip-install the base wheel into a target venv (offline), then copy
           the tracked delta (tools/base_modified.txt + tools/base_added.txt)
           over the installed hermes_agent package. Used by the Windows
           installer's post-install step.

Base sources (dev mode), in priority order:
  --base-dir DIR    an extracted fork checkout (contains pyproject.toml, agent/, ...)
  --base-tarball F   a github *.tar.gz of the fork commit (extracted to a tempdir)
  --base-wheel F     a hermes_agent-*.whl (extracted to a tempdir; package files
                    land under hermes_agent-<ver>/ in the tempdir root, so DIR
                    is mapped accordingly)
  otherwise: download https://github.com/LynxLabVN/agent-core/archive/<sha>.tar.gz

Usage
-----
  python tools/provision_base.py                    # dev: auto-download base
  python tools/provision_base.py --base-dir /path/to/fork-checkout
  python tools/provision_base.py install --base-wheel dist/hermes_agent-0.18.0-py3-none-any.whl \
                       --venv C:/path/to/venv --delta-root .
"""
from __future__ import annotations

import argparse
import os
import shutil
import subprocess
import sys
import tarfile
import tempfile
import urllib.request

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
AGENT_CORE = os.path.join(ROOT, "agent-core")
BASE_COMMIT = "88d1d6206f399c134d1f4c0b7db27733aaa3c50c"
TARBALL_URL = f"https://github.com/LynxLabVN/agent-core/archive/{BASE_COMMIT}.tar.gz"


def read_list(name: str) -> list[str]:
    with open(os.path.join(HERE, name), encoding="utf-8") as f:
        return [ln.strip() for ln in f if ln.strip()]


def _extract_top_dir(path: str) -> str:
    """Return the single top-level dir inside an extracted archive, or path."""
    entries = os.listdir(path)
    if len(entries) == 1 and os.path.isdir(os.path.join(path, entries[0])):
        return os.path.join(path, entries[0])
    return path


def resolve_base_dir(args) -> str:
    if args.base_dir:
        return os.path.abspath(args.base_dir)
    tmp = tempfile.mkdtemp(prefix="hermes-base-")
    tb = args.base_tarball or os.path.join(tmp, "base.tar.gz")
    if args.base_tarball is None:
        print(f"Downloading base from {TARBALL_URL} ...")
        urllib.request.urlretrieve(TARBALL_URL, tb)
    with tarfile.open(tb) as t:
        t.extractall(tmp)
    return _extract_top_dir(tmp)


def cmd_dev(args) -> int:
    base = resolve_base_dir(args)
    files = read_list("base_files.txt")
    missing = []
    copied = 0
    for rel in files:
        src = os.path.join(base, rel.replace("/", os.sep))
        dst = os.path.join(AGENT_CORE, rel.replace("/", os.sep))
        if not os.path.isfile(src):
            missing.append(rel)
            continue
        os.makedirs(os.path.dirname(dst), exist_ok=True)
        shutil.copy2(src, dst)
        copied += 1
    print(f"Restored {copied}/{len(files)} base files into agent-core/")
    if missing:
        print(f"WARNING: {len(missing)} base files not found in source; e.g.:")
        for m in missing[:5]:
            print(f"  - {m}")
    # sanity: the tracked delta must still be present
    for rel in read_list("base_modified.txt") + read_list("base_added.txt"):
        if not os.path.isfile(os.path.join(AGENT_CORE, rel.replace("/", os.sep))):
            print(f"WARNING: delta file missing after provision: {rel}")
    print("Done. agent-core/ now = Hermes base + office-agent delta.")
    print("Next: cd agent-core && uv pip install -e '.[mcp,acp,dev]'")
    return 0 if not missing else 2


def cmd_install(args) -> int:
    venv = os.path.abspath(args.venv)
    py = os.path.join(venv, "Scripts", "python.exe")
    if not os.path.isfile(py):
        py = os.path.join(venv, "bin", "python")
    wheel = os.path.abspath(args.base_wheel)
    # Offline install of the base wheel + its deps from a local wheels dir.
    find_links = ["--find-links", os.path.dirname(wheel)]
    subprocess.check_call(
        [py, "-m", "pip", "install", "--no-index", *find_links, wheel]
    )
    # Locate the installed hermes_agent package dir.
    site = subprocess.check_output(
        [py, "-c", "import hermes_cli, os; print(os.path.dirname(hermes_cli.__file__))"],
        text=True,
    ).strip()
    pkg_root = os.path.dirname(site)  # .../site-packages
    delta_root = os.path.abspath(args.delta_root)
    n = 0
    for rel in read_list("base_modified.txt") + read_list("base_added.txt"):
        src = os.path.join(delta_root, "agent-core", rel.replace("/", os.sep))
        dst = os.path.join(pkg_root, rel.replace("/", os.sep))
        if not os.path.isfile(src):
            print(f"skip (not in delta tree): {rel}")
            continue
        os.makedirs(os.path.dirname(dst), exist_ok=True)
        shutil.copy2(src, dst)
        n += 1
    print(f"Overlaid {n} delta files onto {pkg_root}")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    sub = ap.add_subparsers(dest="mode", required=True)
    d = sub.add_parser("dev", help="restore base files into agent-core/")
    d.add_argument("--base-dir")
    d.add_argument("--base-tarball")
    d.set_defaults(func=cmd_dev)
    i = sub.add_parser("install", help="pip-install base wheel + overlay delta")
    i.add_argument("--base-wheel", required=True)
    i.add_argument("--venv", required=True)
    i.add_argument("--delta-root", default=".", help="repo root (contains agent-core/)")
    i.set_defaults(func=cmd_install)
    args = ap.parse_args()
    return args.func(args)


if __name__ == "__main__":
    sys.exit(main())