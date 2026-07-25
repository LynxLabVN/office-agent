"""PDF/image CV parsing using PyMuPDF with heuristic field extraction."""

from __future__ import annotations

import re
from pathlib import Path
from typing import Any


def _extract_email(text: str) -> str | None:
    match = re.search(
        r"[\w.+-]+@[\w-]+\.[\w.-]+", text, flags=re.IGNORECASE
    )
    return match.group(0) if match else None


def _extract_phone(text: str) -> str | None:
    # Vietnamese-centric phone formats; also accepts generic international.
    patterns = [
        r"(?:\+84|84|0)\d{9,10}",
        r"\+\d{10,14}",
    ]
    for pat in patterns:
        match = re.search(pat, text.replace(" ", "").replace("-", "").replace(".", ""))
        if match:
            return match.group(0)
    return None


def _extract_exp_years(text: str) -> int:
    # Look for patterns like "5 years", "5+ years of experience", "5 yrs".
    matches = re.findall(
        r"(\d+(?:\.\d+)?)\+?\s*(?:years?|yrs?)\s*(?:of\s*)?experience",
        text,
        flags=re.IGNORECASE,
    )
    if matches:
        return int(float(matches[0]))
    # Fallback: total X years in summary.
    match = re.search(
        r"(\d+(?:\.\d+)?)\+?\s*years?", text, flags=re.IGNORECASE
    )
    if match:
        return int(float(match.group(1)))
    return 0


def _extract_education(text: str) -> list[dict[str, str]]:
    edu_keywords = [
        r"bachelor",
        r"master",
        r"phd",
        r"doctorate",
        r"associate",
        r"b\.s",
        r"m\.s",
        r"b\.eng",
        r"m\.eng",
        r"computer science",
        r"information technology",
    ]
    results: list[dict[str, str]] = []
    for line in text.splitlines():
        line_lower = line.lower()
        if any(kw in line_lower for kw in edu_keywords):
            results.append({"line": line.strip()})
    return results


def _extract_name(text: str) -> str | None:
    lines = [ln.strip() for ln in text.splitlines() if ln.strip()]
    for line in lines[:20]:
        # Skip lines that look like contact info, URLs, section headers.
        if re.search(r"@|http|tel|phone|email|cv|resume|curriculum vitae", line, re.I):
            continue
        words = line.split()
        if 2 <= len(words) <= 4 and all(w.isalpha() or w in "-." for w in words):
            # Heuristic: title-case or all-caps name.
            if line.istitle() or line.isupper():
                return line.title()
    return None


def parse_pdf(file_path: str) -> dict[str, Any]:
    """Parse a PDF file and return structured CV fields."""
    try:
        import fitz  # PyMuPDF
    except ImportError as exc:  # pragma: no cover
        raise RuntimeError("PyMuPDF is required to parse PDFs") from exc

    path = Path(file_path)
    if not path.exists():
        raise FileNotFoundError(f"PDF not found: {file_path}")

    text_parts: list[str] = []
    doc = fitz.open(str(path))
    for page in doc:
        text_parts.append(page.get_text("text"))
    doc.close()

    raw_text = "\n".join(text_parts).strip()

    name = _extract_name(raw_text)
    email = _extract_email(raw_text)
    phone = _extract_phone(raw_text)
    exp_years = _extract_exp_years(raw_text)
    education = _extract_education(raw_text)

    return {
        "name": name,
        "contact": {
            "email": email,
            "phone": phone,
        },
        "exp_years": exp_years,
        "skills": [],  # populated by extract_skills / server layer
        "education": education,
        "raw_text": raw_text,
    }
