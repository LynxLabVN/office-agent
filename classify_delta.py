#!/usr/bin/env python3
"""Classify office-agent/agent-core files vs the vendored Hermes baseline.

For each tracked file under agent-core/:
  - identical : same path exists in baseline with identical content (CRLF-normalized)
  - modified  : same path exists but content differs (CRLF-normalized)
  - added     : not present in the baseline (office-agent addition)

Line endings are normalized (\\r\\n -> \\n) before comparing so a Windows
checkout doesn't get flagged as 'modified' just because of CRLF.

Usage: python classify_delta.py <baseline_dir> <office_agent_root>
Writes /tmp/oai-classify/{identical,modified,added}.txt
"""
import os
import subprocess
import sys

BASE = sys.argv[1]
ROOT = sys.argv[2]
OUT = "/tmp/oai-classify2"
os.makedirs(OUT, exist_ok=True)


def norm(path):
    try:
        with open(path, "rb") as f:
            return f.read().replace(b"\r\n", b"\n")
    except IsADirectoryError:
        return None
    except OSError:
        return None


def main():
    files = subprocess.check_output(
        ["git", "-C", ROOT, "ls-files", "agent-core"], text=True
    ).splitlines()
    ident = mod = add = 0
    with open(f"{OUT}/identical.txt", "w") as fi, \
         open(f"{OUT}/modified.txt", "w") as fm, \
         open(f"{OUT}/added.txt", "w") as fa:
        for f in files:
            rel = f[len("agent-core/"):] if f.startswith("agent-core/") else f
            ours = os.path.join(ROOT, "agent-core", rel.replace("/", os.sep))
            base = os.path.join(BASE, rel.replace("/", os.sep))
            if not os.path.isfile(base):
                fa.write(rel + "\n"); add += 1
                continue
            o = norm(ours); b = norm(base)
            if o is None or b is None:
                fm.write(rel + "\n"); mod += 1
            elif o == b:
                fi.write(rel + "\n"); ident += 1
            else:
                fm.write(rel + "\n"); mod += 1
    print(f"identical: {ident}")
    print(f"modified:  {mod}")
    print(f"added:     {add}")
    print(f"total:     {ident + mod + add}")


if __name__ == "__main__":
    main()