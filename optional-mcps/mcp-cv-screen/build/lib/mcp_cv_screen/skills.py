"""Skill extraction and normalization from raw text."""

from __future__ import annotations

import re
from typing import Any

# Expanded keyword map for A/V, event-production, and general tech roles.
SKILL_KEYWORDS: dict[str, list[str]] = {
    "audio engineering": ["audio engineer", "sound engineer", "audio mixing", "live sound"],
    "video editing": ["video edit", "premiere", "final cut", "davinci resolve"],
    "ffmpeg": ["ffmpeg"],
    "python": ["python"],
    "rust": ["rust"],
    "javascript": ["javascript", "js"],
    "typescript": ["typescript", "ts"],
    "react": ["react", "reactjs"],
    "led mapping": ["led mapping", "madmapper", "resolume", "pixera"],
    "lighting design": ["lighting design", "lighting operator", "grandma", "ma lighting"],
    "project management": ["project management", "project manager", "scrum", "agile"],
    "event production": ["event production", "event producer", "stage management"],
    "photoshop": ["photoshop", "adobe photoshop"],
    "illustrator": ["illustrator", "adobe illustrator"],
    "after effects": ["after effects", "ae"],
    "blender": ["blender"],
    "unity": ["unity"],
    "unreal engine": ["unreal engine", "unreal"],
    "touchdesigner": ["touchdesigner"],
    "qlab": ["qlab"],
    "sql": ["sql", "mysql", "postgresql", "sqlite"],
    "docker": ["docker"],
    "kubernetes": ["kubernetes", "k8s"],
    "aws": ["aws", "amazon web services"],
    "linux": ["linux"],
    "git": ["git"],
    "ci/cd": ["ci/cd", "github actions", "gitlab ci"],
    "english": ["english"],
    "vietnamese": ["vietnamese"],
}


def normalize(name: str) -> str:
    return " ".join(name.lower().strip().split())


def extract(text: str) -> list[dict[str, Any]]:
    """Return normalized skills with confidence scores (0.0-1.0)."""
    text_lower = text.lower()
    found: list[dict[str, Any]] = []

    for skill_name, aliases in SKILL_KEYWORDS.items():
        max_confidence = 0.0
        for alias in aliases:
            pattern = r"(?:^|[\s,;()])" + re.escape(alias) + r"(?:[\s,;()]|$)"
            matches = list(re.finditer(pattern, text_lower))
            if matches:
                # Multiple mentions increase confidence up to 1.0.
                confidence = min(0.6 + 0.1 * (len(matches) - 1), 1.0)
                max_confidence = max(max_confidence, confidence)
        if max_confidence > 0:
            found.append(
                {
                    "name": skill_name,
                    "normalized": normalize(skill_name),
                    "confidence": round(max_confidence, 2),
                }
            )

    # Sort by confidence descending.
    found.sort(key=lambda x: x["confidence"], reverse=True)
    return found
