"""Unit tests for the VN labor-law offer compliance checker (Phase 5.8)."""

from __future__ import annotations

from skills.hr.compliance import check_offer_compliance


COMPLETE_OFFER = """
Chào mừng bạn gia nhập đội ngũ LynxLab.

Vị trí: Senior Video Editor
Loại hợp đồng: Không xác định thời hạn
Thử việc: 30 ngày
Lương: 25 triệu VND/tháng

BHXH, BHYT, BHTN theo quy định nhà nước (nhà tuyển dụng đóng 17.5%,
người lao động đóng 8%).

Thời gian làm việc: 8h/ngày, 48h/tuần.
Làm thêm (overtime): 1.5 lần lương ngày thường, 2 lần cuối tuần,
3 lần ngày lễ theo Bộ luật Lao động.
"""


def _names(result) -> list[str]:
    return [c["name"] for c in result["checks"]]


def test_complete_offer_passes():
    result = check_offer_compliance(COMPLETE_OFFER)
    failed = [c["name"] for c in result["checks"] if not c["passed"]]
    assert result["passed"] is True, f"failed checks: {failed}"
    assert result["missing_required"] == []
    # Every required checklist item is present and passing.
    expected = {
        "probation_max_60_days",
        "contract_type_specified",
        "si_hi_ui_noted",
        "working_hours_within_limit",
        "overtime_premium_rates",
    }
    assert expected <= set(_names(result))


def test_missing_probation_clause_is_flagged():
    """An offer without a probation clause must be flagged for human review."""
    offer = COMPLETE_OFFER.replace("Thử việc: 30 ngày\n", "")
    result = check_offer_compliance(offer)

    assert result["passed"] is False
    assert "probation_max_60_days" in result["missing_required"]
    names = _names(result)
    probation = next(c for c in result["checks"] if c["name"] == "probation_max_60_days")
    assert probation["passed"] is False


def test_probation_over_60_days_is_flagged():
    """Probation longer than 60 days violates Labor Code Art 27."""
    offer = COMPLETE_OFFER.replace("Thử việc: 30 ngày", "Thử việc: 90 ngày")
    result = check_offer_compliance(offer)
    assert result["passed"] is False
    assert "probation_max_60_days" in result["missing_required"]


def test_missing_si_contributions_flagged():
    offer = COMPLETE_OFFER.replace(
        "BHXH, BHYT, BHTN theo quy định nhà nước (nhà tuyển dụng đóng 17.5%,\n"
        "người lao động đóng 8%).",
        "Các khoản bảo hiểm được thảo luận riêng.",
    )
    result = check_offer_compliance(offer)
    assert result["passed"] is False
    assert "si_hi_ui_noted" in result["missing_required"]


def test_missing_overtime_rates_flagged():
    offer = COMPLETE_OFFER.replace(
        "Làm thêm (overtime): 1.5 lần lương ngày thường, 2 lần cuối tuần,\n"
        "3 lần ngày lễ theo Bộ luật Lao động.",
        "Làm thêm theo thoả thuận.",
    )
    result = check_offer_compliance(offer)
    assert "overtime_premium_rates" in result["missing_required"]