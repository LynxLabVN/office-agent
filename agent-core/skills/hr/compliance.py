"""Vietnam labor-law compliance checker for drafted offer letters."""

from __future__ import annotations

import re
from dataclasses import dataclass
from typing import Any, Dict, List, Optional


@dataclass
class ComplianceCheck:
    name: str
    passed: bool
    required: bool
    note: str


# Vietnamese number patterns (digits, triệu, etc.) and time patterns.
_SI_PATTERN = re.compile(
    r"(BHXH|BHYT|BHTN|bảo hiểm xã hội|bảo hiểm y tế|bảo hiểm thất nghiệp|17\.?5%|8%)",
    re.IGNORECASE,
)
_HOURS_PATTERN = re.compile(r"(\d+)\s*(h|giờ)\s*/\s*(ngày|day)", re.IGNORECASE)
_OVERTIME_PATTERN = re.compile(
    r"(1\.5\s*x?\s*(lần|x)|2\s*x?\s*(lần|x)|3\s*x?\s*(lần|x)|overtime|làm thêm)",
    re.IGNORECASE,
)


def check_offer_compliance(offer_text: str) -> Dict[str, Any]:
    """Check a drafted offer letter against VN labor law basics."""
    text = offer_text.lower()
    checks: List[ComplianceCheck] = []

    # Probation ≤ 60 days (Labor Code Art 27)
    probation_days: Optional[int] = None
    for m in re.finditer(r"(thử việc|probation).*?(\d+)\s*(ngày|days?)", text):
        try:
            probation_days = int(m.group(2))
            break
        except (IndexError, ValueError):
            pass
    checks.append(
        ComplianceCheck(
            name="probation_max_60_days",
            passed=probation_days is not None and probation_days <= 60,
            required=True,
            note=f"Probation {probation_days} days" if probation_days is not None else "Probation clause missing",
        )
    )

    # Contract type specified
    contract_types = ["xác định thờ hạn", "không xác định thờ hạn", "mùa vụ", "theo mùa vụ"]
    has_contract_type = any(ct in text for ct in contract_types)
    checks.append(
        ComplianceCheck(
            name="contract_type_specified",
            passed=has_contract_type,
            required=True,
            note="Contract type found" if has_contract_type else "Contract type missing",
        )
    )

    # SI/HC/HI contributions noted
    has_si = bool(_SI_PATTERN.search(offer_text))
    checks.append(
        ComplianceCheck(
            name="si_hi_ui_noted",
            passed=has_si,
            required=True,
            note="SI/HI/UI contributions noted" if has_si else "SI/HI/UI contributions missing",
        )
    )

    # Working hours ≤ 8h/day, 48h/week
    hours_match = _HOURS_PATTERN.search(offer_text)
    daily_hours = int(hours_match.group(1)) if hours_match else None
    checks.append(
        ComplianceCheck(
            name="working_hours_within_limit",
            passed=daily_hours is not None and daily_hours <= 8,
            required=True,
            note=f"Daily hours: {daily_hours}" if daily_hours is not None else "Working hours missing",
        )
    )

    # Overtime caps + premium rates
    has_overtime_terms = bool(_OVERTIME_PATTERN.search(offer_text))
    checks.append(
        ComplianceCheck(
            name="overtime_premium_rates",
            passed=has_overtime_terms,
            required=True,
            note="Overtime premium rates noted" if has_overtime_terms else "Overtime premium rates missing",
        )
    )

    missing = [c.name for c in checks if c.required and not c.passed]
    return {
        "passed": len(missing) == 0,
        "missing_required": missing,
        "checks": [
            {"name": c.name, "passed": c.passed, "required": c.required, "note": c.note}
            for c in checks
        ],
    }
