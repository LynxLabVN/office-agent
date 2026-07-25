"""CV scoring rubric against a job description."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

from mcp_cv_screen import skills as skills_mod


def _load_jd(jd_id: str, cache_dir: Path) -> dict[str, Any] | None:
    """Resolve a JD identifier to a dict.

    Resolution order:
    1. Parse ``jd_id`` as inline JSON.
    2. Load from ``cache_dir/jds/<jd_id>.json``.
    3. Load from ``jd_id`` as a filesystem path.
    """
    try:
        data = json.loads(jd_id)
        if isinstance(data, dict):
            return data
    except json.JSONDecodeError:
        pass

    cached = cache_dir / "jds" / f"{jd_id}.json"
    if cached.exists():
        return json.loads(cached.read_text(encoding="utf-8"))

    path = Path(jd_id)
    if path.exists():
        text = path.read_text(encoding="utf-8")
        try:
            return json.loads(text)
        except json.JSONDecodeError:
            return {"raw_text": text}
    return None


def _jd_skills(jd: dict[str, Any]) -> list[str]:
    skills: list[str] = []
    for key in ("skills_required", "skills_nice", "skills"):
        value = jd.get(key)
        if isinstance(value, list):
            skills.extend(str(s).lower() for s in value)
        elif isinstance(value, str):
            skills.extend(s.strip().lower() for s in value.split(",") if s.strip())
    # Also scan raw JD text for known skill keywords.
    raw = jd.get("raw_text") or jd.get("jd_markdown") or ""
    extracted = skills_mod.extract(raw)
    skills.extend(e["normalized"] for e in extracted)
    return list(dict.fromkeys(skills))


def _jd_exp_level(jd: dict[str, Any]) -> str:
    raw = str(jd.get("exp_level", "")).lower()
    return raw or ""


def _score_skills(cv_skills: list[str], jd_skills: list[str]) -> float:
    if not jd_skills:
        return 0.0
    cv_set = set(cv_skills)
    hits = sum(1 for s in jd_skills if any(s in cv_s or cv_s in s for cv_s in cv_set))
    return hits / len(jd_skills)


def _score_exp(exp_years: int, jd: dict[str, Any]) -> float:
    level = _jd_exp_level(jd)
    if not level:
        return 0.5
    expected = {"junior": 1, "mid": 3, "senior": 5, "lead": 7}.get(level, 3)
    if exp_years >= expected:
        return 1.0
    return max(0.0, exp_years / expected)


def _score_edu(education: list[Any]) -> float:
    return 1.0 if education else 0.3


def _score_portfolio(portfolio_urls: list[str]) -> float:
    return 1.0 if portfolio_urls else 0.0


def score(cv: dict[str, Any], jd_id: str, cache_dir: Path | None = None) -> dict[str, Any]:
    """Score a parsed CV against a JD.

    ``cv`` is the parsed CV dict from :func:`mcp_cv_screen.parse.parse_pdf`.
    ``jd_id`` may be inline JSON, a cache key, or a file path.
    """
    cache_dir = cache_dir or Path.home() / ".hermes" / "data" / "cv_cache"
    jd = _load_jd(jd_id, cache_dir)
    if jd is None:
        return {
            "score": 0,
            "breakdown": {"skills": 0, "exp": 0, "portfolio": 0, "edu": 0},
            "error": f"jd_id not resolved: {jd_id}",
        }

    cv_skills = [s.lower() for s in cv.get("skills", [])]
    jd_skills = _jd_skills(jd)

    skills_score = _score_skills(cv_skills, jd_skills)
    exp_score = _score_exp(cv.get("exp_years", 0), jd)
    edu_score = _score_edu(cv.get("education", []))
    portfolio_score = _score_portfolio(cv.get("portfolio_urls", []))

    # Rubric: skills 40%, exp 30%, portfolio 20%, edu 10%.
    total = (
        skills_score * 0.4
        + exp_score * 0.3
        + portfolio_score * 0.2
        + edu_score * 0.1
    )

    return {
        "score": round(total * 100),
        "breakdown": {
            "skills": round(skills_score * 100),
            "exp": round(exp_score * 100),
            "portfolio": round(portfolio_score * 100),
            "edu": round(edu_score * 100),
        },
    }
