"""Unit tests for the reply-policy skill.

Run with pytest if available:
    python -m pytest test_policy.py -v

Otherwise run directly:
    python test_policy.py
"""

from __future__ import annotations

import os
import sys
import traceback

from policy import decide_reply, load_templates, llm_guard, match_template


TEMPLATES_PATH = os.path.join(os.path.dirname(__file__), "templates.toml")


def test_load_templates_returns_dict():
    templates = load_templates(TEMPLATES_PATH)
    assert isinstance(templates, dict)
    assert "youtube" in templates
    assert "zalo_oa" in templates


def test_match_template_all_five_scenarios():
    templates = load_templates(TEMPLATES_PATH)
    cases = [
        ("youtube", "youtube_thank_you"),
        ("zalo_oa", "zalo_oa_question"),
        ("zalo_oa", "zalo_oa_human_handoff"),
        ("facebook", "social_comment_dm"),
        ("generic", "reject_unsafe"),
    ]
    for platform, scenario in cases:
        template = match_template(platform, scenario, "hello", templates)
        assert template is not None, f"missing template for {platform}/{scenario}"
        assert template["platform"] == platform
        assert template["scenario"] == scenario
        assert isinstance(template["text"], str)


def test_match_template_unknown_returns_none():
    templates = load_templates(TEMPLATES_PATH)
    assert match_template("unknown", "nope", "hello", templates) is None


def test_llm_guard_rejects_guarantee():
    result = llm_guard("We guarantee a full refund.", {})
    assert result["approved"] is False
    assert "promise" in result["reason"].lower()


def test_llm_guard_rejects_phone_number():
    result = llm_guard("Call me at 0901234567 for details.", {})
    assert result["approved"] is False
    assert "phone" in result["reason"].lower()


def test_llm_guard_rejects_email():
    result = llm_guard("Reach me at user@example.com", {})
    assert result["approved"] is False
    assert "email" in result["reason"].lower()


def test_llm_guard_rejects_long_reply():
    result = llm_guard("x" * 201, {})
    assert result["approved"] is False
    assert "characters" in result["reason"].lower()


def test_llm_guard_rejects_salary_in_hr():
    result = llm_guard("Mức lương 15 triệu một tháng", {"domain": "hr"})
    assert result["approved"] is False
    assert "salary" in result["reason"].lower()


def test_llm_guard_approves_safe_reply():
    result = llm_guard("Cảm ơn bạn đã quan tâm!", {})
    assert result["approved"] is True


def test_decide_reply_suggest_mode_queues():
    templates = load_templates(TEMPLATES_PATH)
    result = decide_reply(
        inbound="thanks",
        platform="youtube",
        scenario="youtube_thank_you",
        mode="suggest",
        templates=templates,
        context={},
    )
    assert result["action"] == "queue_human"
    assert result["reply"] is not None
    assert "human approval" in result["reason"].lower()


def test_decide_reply_auto_mode_sends_safe_template():
    templates = load_templates(TEMPLATES_PATH)
    result = decide_reply(
        inbound="thanks",
        platform="youtube",
        scenario="youtube_thank_you",
        mode="auto",
        templates=templates,
        context={},
    )
    assert result["action"] == "send"
    assert result["reply"] is not None


def test_decide_reply_auto_mode_sends_safe_generic_template():
    templates = load_templates(TEMPLATES_PATH)
    result = decide_reply(
        inbound="unsafe",
        platform="generic",
        scenario="reject_unsafe",
        mode="auto",
        templates=templates,
        context={},
    )
    assert result["action"] == "send"
    assert result["reply"] is not None


def test_decide_reply_auto_mode_queues_unsafe_reply():
    templates = {
        "youtube": {
            "promo": {
                "text": "We guarantee the lowest price! Call 0901234567.",
                "labels": ["promo"],
            }
        }
    }
    result = decide_reply(
        inbound="promo",
        platform="youtube",
        scenario="promo",
        mode="auto",
        templates=templates,
        context={},
    )
    assert result["action"] == "queue_human"
    assert result["reply"] is not None
    assert "guard rejected" in result["reason"].lower()


def test_decide_reply_off_mode_drops():
    templates = load_templates(TEMPLATES_PATH)
    result = decide_reply(
        inbound="thanks",
        platform="youtube",
        scenario="youtube_thank_you",
        mode="off",
        templates=templates,
        context={},
    )
    assert result["action"] == "drop"
    assert result["reply"] is None


# ---- Direct-execution fallback for environments without pytest ----

def _run_direct():
    tests = [
        (name, obj)
        for name, obj in globals().items()
        if name.startswith("test_") and callable(obj)
    ]
    passed = failed = 0
    for name, test in tests:
        try:
            test()
            print(f"  PASS {name}")
            passed += 1
        except Exception as exc:
            failed += 1
            print(f"  FAIL {name}: {exc}")
            traceback.print_exc()
    print(f"\n{passed} passed, {failed} failed")
    sys.exit(0 if failed == 0 else 1)


if __name__ == "__main__":
    try:
        import pytest  # type: ignore[import-not-found]
    except ImportError:
        pytest = None  # type: ignore[assignment]

    if pytest is None:
        _run_direct()
    else:
        sys.exit(pytest.main([__file__, "-v"]))
