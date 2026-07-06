"""Unit tests for the auto-edit quality scorer (Phase 5.5)."""

from __future__ import annotations

from skills.marketing.quality_scorer import (
    ALWAYS_HUMAN_FORMATS,
    score_render_with_metrics,
)


def test_good_render_passes():
    """A well-formed short-form render scores high and is auto-approved."""
    report = score_render_with_metrics(
        "short",
        duration=12.0,
        has_text_overlay=True,
        has_motion=True,
        has_clear_subject=True,
        input_tp=-2.0,
        input_i=-16.0,
        input_lra=8.0,
        caption_confidence=0.95,
        width=1080,
        height=1920,
        fps=30.0,
        has_audio_stream=True,
    )
    assert report.score > 70, report.details
    assert report.passed is True
    assert report.requires_human_review is False
    assert "auto-edit low quality" not in report.flags


def test_bad_render_is_flagged():
    """A render with clipping + no hook text + low captions scores <60 and is flagged."""
    report = score_render_with_metrics(
        "short",
        duration=12.0,
        has_text_overlay=False,
        has_motion=False,
        has_clear_subject=False,
        input_tp=2.0,  # clipping: true peak > 0 dBTP
        input_i=-8.0,
        input_lra=18.0,
        caption_confidence=0.4,
        width=1080,
        height=1920,
        fps=30.0,
        has_audio_stream=True,
    )
    assert report.score < 60, report.score
    assert report.passed is False
    assert report.requires_human_review is True
    assert "auto-edit low quality" in report.flags


def test_clipping_alone_reduces_audio_score():
    """Clipping drives the audio sub-score to zero."""
    good = score_render_with_metrics("short", input_tp=-2.0)
    clipping = score_render_with_metrics("short", input_tp=2.0)
    assert clipping.details["audio"]["score"] < good.details["audio"]["score"]
    assert clipping.details["audio"]["score"] == 0


def test_skit_format_always_requires_human_review():
    """Storytelling/skits are always flagged for human review regardless of score."""
    for fmt in ALWAYS_HUMAN_FORMATS:
        report = score_render_with_metrics(
            fmt,
            duration=12.0,
            has_text_overlay=True,
            has_motion=True,
            has_clear_subject=True,
            input_tp=-2.0,
            caption_confidence=0.95,
        )
        assert report.requires_human_review is True, fmt
        assert report.passed is False, fmt


def test_strong_auto_format_not_force_flagged():
    """A strong-auto format with a good render is not forced to human review."""
    report = score_render_with_metrics(
        "demo",
        duration=12.0,
        has_text_overlay=True,
        has_motion=True,
        has_clear_subject=True,
        input_tp=-2.0,
        caption_confidence=0.9,
    )
    assert report.requires_human_review is False
    assert report.passed is True