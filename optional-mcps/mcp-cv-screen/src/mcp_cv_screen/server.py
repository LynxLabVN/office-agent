import json
import os
from hashlib import sha256
from pathlib import Path

from fastmcp import FastMCP

from mcp_cv_screen import parse as parse_mod
from mcp_cv_screen import portfolio as portfolio_mod
from mcp_cv_screen import score as score_mod
from mcp_cv_screen import skills as skills_mod

mcp = FastMCP("mcp-cv-screen")


def _cache_dir() -> Path:
    raw = os.environ.get("CV_CACHE_DIR", "~/.hermes/data/cv_cache")
    return Path(raw).expanduser()


def _cv_id_for(file_path: str) -> str:
    return sha256(file_path.encode()).hexdigest()[:16]


def _cv_cache_path(cv_id: str) -> Path:
    cache = _cache_dir() / "cvs"
    cache.mkdir(parents=True, exist_ok=True)
    return cache / f"{cv_id}.json"


@mcp.tool()
def health() -> str:
    """Return server health status."""
    return json.dumps({"ok": True, "name": "mcp-cv-screen"})


@mcp.tool()
def parse_cv(file_path: str) -> str:
    """Parse a CV PDF into structured fields.

    Returns name, contact, years of experience, skills, education, and raw text.
    The result is cached under ``CV_CACHE_DIR`` using a stable id derived from
    ``file_path`` so downstream tools such as ``score_cv_against_jd`` can load it.
    """
    parsed = parse_mod.parse_pdf(file_path)
    parsed["skills"] = [e["name"] for e in skills_mod.extract(parsed["raw_text"])]

    cv_id = _cv_id_for(file_path)
    parsed["cv_id"] = cv_id
    parsed["file_path"] = file_path

    cache_path = _cv_cache_path(cv_id)
    cache_path.write_text(json.dumps(parsed, ensure_ascii=False, indent=2), encoding="utf-8")

    return json.dumps(parsed, ensure_ascii=False)


@mcp.tool()
def extract_skills(text: str) -> str:
    """Extract normalized skills from text.

    Uses a keyword map plus simple normalization. Tools that need LLM-based
    normalization should pass the returned list to the Hermes agent for review.
    """
    skills = skills_mod.extract(text)
    return json.dumps({"skills": skills, "text_sample": text[:200]})


@mcp.tool()
def score_cv_against_jd(cv_id: str, jd_id: str) -> str:
    """Score a CV against a job description.

    ``cv_id`` may be the id returned by ``parse_cv`` or a path to a cached JSON
    file. ``jd_id`` may be inline JSON, a cache key under ``CV_CACHE_DIR/jds/``,
    or a file path containing JD text/JSON.
    """
    cache_dir = _cache_dir()
    cv_path = _cv_cache_path(cv_id)
    if not cv_path.exists():
        alt = Path(cv_id)
        if alt.exists():
            cv = json.loads(alt.read_text(encoding="utf-8"))
        else:
            return json.dumps(
                {
                    "score": 0,
                    "breakdown": {"skills": 0, "exp": 0, "portfolio": 0, "edu": 0},
                    "error": f"cv_id not found: {cv_id}",
                }
            )
    else:
        cv = json.loads(cv_path.read_text(encoding="utf-8"))

    result = score_mod.score(cv, jd_id, cache_dir=cache_dir)
    return json.dumps(result)


@mcp.tool()
def analyze_portfolio(url_or_files: list[str]) -> str:
    """Analyze portfolio URLs or local media files.

    For web URLs, fetches text via httpx. For video files, transcribes audio
    using faster-whisper (model configurable via ``WHISPER_MODEL`` env var).
    """
    whisper_model = os.environ.get("WHISPER_MODEL", "base")
    result = portfolio_mod.analyze(url_or_files, whisper_model=whisper_model)
    return json.dumps(result)


@mcp.tool()
def compare_candidates(cv_ids: list[str], jd_id: str) -> str:
    """Compare multiple candidates against a JD and return ranked results."""
    cache_dir = _cache_dir()
    rankings: list[dict] = []

    for cv_id in cv_ids:
        cv_path = _cv_cache_path(cv_id)
        if not cv_path.exists():
            continue
        cv = json.loads(cv_path.read_text(encoding="utf-8"))
        scored = score_mod.score(cv, jd_id, cache_dir=cache_dir)
        rankings.append(
            {
                "cv_id": cv_id,
                "name": cv.get("name") or cv_id,
                "score": scored.get("score", 0),
            }
        )

    rankings.sort(key=lambda x: x["score"], reverse=True)
    for i, row in enumerate(rankings, start=1):
        row["rank"] = i

    return json.dumps(rankings)


@mcp.tool()
def summarize_profile(cv_id: str) -> str:
    """Summarize a candidate profile in three lines.

    When parsed data is available, returns a rule-based summary. The response
    also includes a suggested prompt so the Hermes agent can refine it via LLM.
    """
    cv_path = _cv_cache_path(cv_id)
    if not cv_path.exists():
        return json.dumps(
            {
                "summary": "Candidate profile not found.",
                "cv_id": cv_id,
                "prompt_for_agent": None,
            }
        )

    cv = json.loads(cv_path.read_text(encoding="utf-8"))
    name = cv.get("name") or "Candidate"
    exp = cv.get("exp_years", 0)
    skill_names = cv.get("skills", [])
    skills_text = ", ".join(skill_names[:5]) or "various skills"
    education_count = len(cv.get("education", []))
    edu_text = "has relevant education" if education_count else "education details not extracted"

    summary = (
        f"{name} brings {exp}+ years of experience with core skills in {skills_text}. "
        f"Profile {edu_text}. "
        f"Top skills extracted: {skills_text}."
    )

    prompt = (
        f"Summarize the following candidate in exactly 3 concise lines for a recruiter. "
        f"Name: {name}. Experience: {exp} years. Skills: {skills_text}. "
        f"Raw text excerpt: {cv.get('raw_text', '')[:800]}."
    )

    return json.dumps(
        {
            "summary": summary,
            "cv_id": cv_id,
            "prompt_for_agent": prompt,
        }
    )


if __name__ == "__main__":
    mcp.run(transport="stdio")
