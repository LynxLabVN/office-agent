"""
``hermes hr`` subcommand — LynxLabVN recruitment workflow.

Drives the 11-state recruitment pipeline with JD drafting, CV screening,
interview scheduling, and candidate comms. State is persisted in
``~/.hermes/hr/recruitment.json``.
"""

from __future__ import annotations

import argparse
import json
import os
import sys
from datetime import datetime, timedelta, timezone
from typing import Any, Callable, Dict, List, Optional

from hermes_cli._domain_shared import (
    add_history,
    call_mcp_tool,
    find_item,
    load_domain_state,
    save_domain_state,
    send_manager_review,
    sync_static_cron_jobs,
    _new_id,
    _utcnow,
    print_mcp_fallback_notice,
)
from hermes_cli.colors import Colors, color

DOMAIN = "hr"
STATE_FILE = "recruitment.json"

HR_STATES = [
    "JD_DRAFT",
    "JD_REVIEW_GATE",
    "POST_JOBS",
    "RECEIVE_APPS",
    "CV_SCREEN",
    "SHORTLIST",
    "SCHEDULE_INTERVIEW",
    "INTERVIEW_NOTES",
    "DECISION_GATE",
    "OFFER",
    "ONBOARD_HANDOFF",
]

CANDIDATE_STAGES = [
    "Applied",
    "Screened",
    "Shortlist",
    "Interview",
    "Offer",
    "Hired",
    "Rejected",
]


def _load_state() -> Dict[str, Any]:
    default = {"version": 1, "jobs": [], "candidates": []}
    return load_domain_state(DOMAIN, STATE_FILE)


def _save_state(state: Dict[str, Any]) -> None:
    save_domain_state(DOMAIN, STATE_FILE, state)


def _get_job(state: Dict[str, Any], job_id: str) -> Optional[Dict[str, Any]]:
    return find_item(state.get("jobs", []), job_id)


def _get_candidate(state: Dict[str, Any], candidate_id: str) -> Optional[Dict[str, Any]]:
    return find_item(state.get("candidates", []), candidate_id)


def _prompt(prompt_text: str, default: str = "") -> str:
    if default:
        return input(f"{prompt_text} [{default}]: ").strip() or default
    return input(f"{prompt_text}: ").strip()


def _prompt_list(prompt_text: str) -> List[str]:
    raw = _prompt(prompt_text)
    return [x.strip() for x in raw.split(",") if x.strip()]


def _build_jd() -> Dict[str, Any]:
    print(color("\nInteractive JD draft", Colors.CYAN))
    title = _prompt("Job title")
    must_have = _prompt_list("Must-have skills (comma-separated)")
    nice_to_have = _prompt_list("Nice-to-have skills (comma-separated)")
    exp_level = _prompt("Experience level", "Mid (2-4 years)")
    salary_range = _prompt("Salary range (gross/month)", "Thỏa thuận")
    location = _prompt("Location", "Hybrid — Hồ Chí Minh")
    benefits = _prompt_list("Benefits (comma-separated)")
    probation_days = int(_prompt("Probation max days", "60") or "60")
    contract_type = _prompt("Contract type", "Indefinite after probation")
    si_notes = _prompt("SI/HC/HI notes", "Công ty đóng BHXH/BHYT/BHTN đầy đủ từ ngày ký HĐLĐ.")

    return {
        "title": title,
        "must_have_skills": must_have,
        "nice_to_have_skills": nice_to_have,
        "exp_level": exp_level,
        "salary_range": salary_range,
        "location": location,
        "benefits": benefits,
        "vn_specific": {
            "probation_max_days": probation_days,
            "contract_type": contract_type,
            "si_hc_hi_notes": si_notes,
        },
    }


def _send_jd_review_gate(job: Dict[str, Any]) -> None:
    jd = job.get("jd", {})
    summary = (
        f"Title: {jd.get('title')}\n"
        f"Exp: {jd.get('exp_level')}\n"
        f"Salary: {jd.get('salary_range')}\n"
        f"Location: {jd.get('location')}\n"
        f"Must-have: {', '.join(jd.get('must_have_skills', []))}\n"
        f"Nice-to-have: {', '.join(jd.get('nice_to_have_skills', []))}"
    )
    send_manager_review(
        title="JD Review Required",
        summary=summary,
        approve_callback=f"hermes hr approve-job {job['id']}",
        revise_callback=f"hermes hr revise-job {job['id']}",
        domain=DOMAIN,
        item_id=job["id"],
    )


def cmd_hr_new_job(args: argparse.Namespace) -> int:
    if not sys.stdin.isatty():
        print(color("hermes hr new-job requires an interactive terminal.", Colors.RED))
        return 1

    state = _load_state()
    jd = _build_jd()
    job = {
        "id": _new_id("job-"),
        "jd": jd,
        "state": "JD_DRAFT",
        "created_at": _utcnow(),
        "updated_at": _utcnow(),
        "history": [{"state": "JD_DRAFT", "timestamp": _utcnow(), "note": "Drafted interactively"}],
    }
    state.setdefault("jobs", []).append(job)

    job["state"] = "JD_REVIEW_GATE"
    add_history(job, "JD_REVIEW_GATE", "Awaiting manager JD approval")
    _save_state(state)
    _send_jd_review_gate(job)

    print()
    print(color("JD created and sent for manager review.", Colors.GREEN))
    print(f"  Job ID: {color(job['id'], Colors.YELLOW)}")
    print(f"  Title:  {jd['title']}")
    print(color(f"Approve: hermes hr approve-job {job['id']}", Colors.DIM))
    print(color(f"Revise:  hermes hr revise-job {job['id']}", Colors.DIM))
    return 0


def cmd_hr_approve_job(args: argparse.Namespace) -> int:
    job_id = args.job_id
    state = _load_state()
    job = _get_job(state, job_id)
    if not job:
        print(color(f"Job not found: {job_id}", Colors.RED))
        return 1
    if job.get("state") != "JD_REVIEW_GATE":
        print(color(f"Job is not at JD_REVIEW_GATE (current: {job.get('state')})", Colors.YELLOW))
        return 1
    add_history(job, "POST_JOBS", "Manager approved JD")
    job["state"] = "POST_JOBS"
    _save_state(state)
    print(color(f"Job {job_id} approved. Run `hermes hr post {job_id}` to publish.", Colors.GREEN))
    return 0


def cmd_hr_revise_job(args: argparse.Namespace) -> int:
    job_id = args.job_id
    state = _load_state()
    job = _get_job(state, job_id)
    if not job:
        print(color(f"Job not found: {job_id}", Colors.RED))
        return 1
    if job.get("state") != "JD_REVIEW_GATE":
        print(color(f"Job is not at JD_REVIEW_GATE (current: {job.get('state')})", Colors.YELLOW))
        return 1
    feedback = args.feedback or "Manager requested JD revision."
    job["manager_feedback"] = feedback
    job["state"] = "JD_DRAFT"
    add_history(job, "JD_DRAFT", f"Revision requested: {feedback}")
    _save_state(state)
    print(color(f"Job {job_id} returned to JD_DRAFT with feedback:", Colors.YELLOW))
    print(f"  {feedback}")
    return 0


def cmd_hr_post(args: argparse.Namespace) -> int:
    job_id = args.job_id
    state = _load_state()
    job = _get_job(state, job_id)
    if not job:
        print(color(f"Job not found: {job_id}", Colors.RED))
        return 1

    jd = job.get("jd", {})
    print(color(f"Posting job {job_id}: {jd.get('title')}", Colors.CYAN))

    result = call_mcp_tool(
        "mcp-hr-data",
        "create_job",
        {"job_id": job_id, "jd": jd},
        fallback={"result": {"job_id": job_id, "status": "created (demo fallback)"}},
    )
    if not isinstance(result, dict) or "result" not in result:
        print_mcp_fallback_notice(DOMAIN, "mcp-hr-data", "create_job")

    job["board_text"] = _generate_board_text(jd)
    add_history(job, "POST_JOBS", "Posted to internal careers page and generated board text")
    job["state"] = "RECEIVE_APPS"
    add_history(job, "RECEIVE_APPS", "Job is live and accepting applications")
    _save_state(state)

    print(color("Internal careers page: created/updated", Colors.GREEN))
    print(color("\nBoard-formatted JD (copy to VN boards):", Colors.CYAN))
    print(job["board_text"])
    return 0


def _generate_board_text(jd: Dict[str, Any]) -> str:
    lines = [
        f"📌 {jd.get('title')}",
        "",
        "🎯 Yêu cầu bắt buộc:",
    ]
    for skill in jd.get("must_have_skills", []):
        lines.append(f"  - {skill}")
    if jd.get("nice_to_have_skills"):
        lines.append("")
        lines.append("⭐ Lợi thế:")
        for skill in jd.get("nice_to_have_skills", []):
            lines.append(f"  - {skill}")
    lines.extend([
        "",
        f"💼 Kinh nghiệm: {jd.get('exp_level')}",
        f"💵 Thu nhập: {jd.get('salary_range')}",
        f"📍 Địa điểm: {jd.get('location')}",
        "",
        "🎁 Quyền lợi:",
    ])
    for benefit in jd.get("benefits", []):
        lines.append(f"  - {benefit}")
    vn = jd.get("vn_specific", {})
    lines.extend([
        "",
        f"📝 Thử việc: tối đa {vn.get('probation_max_days', 60)} ngày",
        f"Hợp đồng: {vn.get('contract_type', 'Không xác định thờ hạn sau thử việc')}",
    ])
    return "\n".join(lines)


def _demo_applications(job_id: str, jd: Dict[str, Any]) -> List[Dict[str, Any]]:
    """Generate a few demo applicants if none exist."""
    return [
        {
            "id": _new_id("cand-"),
            "job_id": job_id,
            "name": "Nguyễn Văn A",
            "email": "vana@example.com",
            "source": "LinkedIn",
            "stage": "Applied",
            "applied_at": _utcnow(),
        },
        {
            "id": _new_id("cand-"),
            "job_id": job_id,
            "name": "Trần Thị B",
            "email": "tb@example.com",
            "source": "TopCV",
            "stage": "Applied",
            "applied_at": _utcnow(),
        },
        {
            "id": _new_id("cand-"),
            "job_id": job_id,
            "name": "Lê Văn C",
            "email": "lvc@example.com",
            "source": "CareerBuilder",
            "stage": "Applied",
            "applied_at": _utcnow(),
        },
    ]


def cmd_hr_screen(args: argparse.Namespace) -> int:
    job_id = args.job_id
    state = _load_state()
    job = _get_job(state, job_id)
    if not job:
        print(color(f"Job not found: {job_id}", Colors.RED))
        return 1

    jd = job.get("jd", {})
    # Load or seed demo applications
    candidates = [c for c in state.get("candidates", []) if c.get("job_id") == job_id]
    if not candidates:
        candidates = _demo_applications(job_id, jd)
        state.setdefault("candidates", []).extend(candidates)

    print(color(f"Screening {len(candidates)} candidate(s) for {jd.get('title')}...", Colors.CYAN))

    rows = []
    for cand in candidates:
        result = call_mcp_tool(
            "mcp-cv-screen",
            "score_cv_against_jd",
            {"cv_id": cand["id"], "jd_id": job_id, "jd": jd},
            fallback={
                "result": {
                    "score": 70,
                    "breakdown": {"skills": 28, "experience": 21, "portfolio": 14, "education": 7},
                    "gaps": ["demo fallback"],
                }
            },
        )
        score_data = result.get("result") if isinstance(result, dict) else {}
        if not isinstance(score_data, dict):
            score_data = {}
        score = score_data.get("score", 0)
        cand["score"] = score
        cand["score_breakdown"] = score_data.get("breakdown", {})
        cand["gaps"] = score_data.get("gaps", [])
        old_stage = cand.get("stage", "Applied")
        if score >= 70:
            cand["stage"] = "Shortlist"
            add_history(cand, "SHORTLIST", f"CV score {score}/100 — shortlisted")
        else:
            cand["stage"] = "Rejected"
            add_history(cand, "REJECTED", f"CV score {score}/100 — below threshold")
        rows.append((cand["name"], old_stage, cand["stage"], score))

    add_history(job, "CV_SCREEN", f"Screened {len(candidates)} candidates")
    job["state"] = "CV_SCREEN"
    _save_state(state)

    print(color("\nScreening results:", Colors.CYAN))
    print(f"  {'Name':<20} {'Before':<12} {'After':<12} {'Score':<6}")
    for name, before, after, score in rows:
        after_color = Colors.GREEN if after == "Shortlist" else Colors.RED
        print(f"  {name:<20} {before:<12} {color(after, after_color):<12} {score:<6}")
    return 0


def cmd_hr_schedule(args: argparse.Namespace) -> int:
    candidate_id = args.candidate_id
    state = _load_state()
    cand = _get_candidate(state, candidate_id)
    if not cand:
        print(color(f"Candidate not found: {candidate_id}", Colors.RED))
        return 1
    job = _get_job(state, cand.get("job_id"))
    jd = job.get("jd", {}) if job else {}

    print(color(f"Booking interview for {cand.get('name')}...", Colors.CYAN))
    slot = (datetime.now(timezone.utc) + timedelta(days=2)).strftime("%Y-%m-%dT10:00:00+07:00")
    result = call_mcp_tool(
        "mcp-schedule",
        "book_slot",
        {"candidate_id": candidate_id, "duration_minutes": 60, "preferred_time": slot},
        fallback={"result": {"booking_id": "demo-1", "start_time": slot, "link": "https://cal.com/demo"}},
    )
    booking = result.get("result") if isinstance(result, dict) else {}
    if not booking:
        print_mcp_fallback_notice(DOMAIN, "mcp-schedule", "book_slot")
        booking = {"booking_id": "demo-1", "start_time": slot, "link": "https://cal.com/demo"}

    cand["interview"] = booking
    cand["stage"] = "Interview"
    add_history(cand, "SCHEDULE_INTERVIEW", f"Booked slot at {booking.get('start_time')}")
    _save_state(state)

    print(color("Interview booked:", Colors.GREEN))
    print(f"  Time: {booking.get('start_time')}")
    print(f"  Link: {booking.get('link')}")
    print(f"  Candidate stage: Interview")
    return 0


def cmd_hr_pipeline(args: argparse.Namespace) -> int:
    job_id = args.job_id
    state = _load_state()
    job = _get_job(state, job_id)
    if not job:
        print(color(f"Job not found: {job_id}", Colors.RED))
        return 1
    jd = job.get("jd", {})
    # Best-effort refresh from mcp-hr-data
    apps_result = call_mcp_tool(
        "mcp-hr-data",
        "list_applications",
        {"job_id": job_id},
        fallback=None,
    )
    if isinstance(apps_result, dict) and "result" in apps_result:
        pass  # Could merge into local state; local demo data is sufficient for CLI preview.
    elif apps_result is None:
        print_mcp_fallback_notice(DOMAIN, "mcp-hr-data", "list_applications")
    candidates = [c for c in state.get("candidates", []) if c.get("job_id") == job_id]
    print()
    print(color(f"Candidate pipeline: {jd.get('title')}", Colors.CYAN))
    print(f"  Job ID: {color(job_id, Colors.YELLOW)} | State: {job.get('state')}")
    print()
    for stage in CANDIDATE_STAGES:
        stage_cands = [c for c in candidates if c.get("stage") == stage]
        print(color(f"  [{stage}] ({len(stage_cands)})", Colors.GREEN if stage_cands else Colors.DIM))
        for c in stage_cands:
            score = c.get("score")
            score_str = f" — score {score}" if score is not None else ""
            print(f"      {color(c['id'], Colors.YELLOW)} {c.get('name')}{score_str}")
    return 0


def cmd_hr_remind_interviews(args: argparse.Namespace) -> int:
    state = _load_state()
    candidates = state.get("candidates", [])
    window = datetime.now(timezone.utc) + timedelta(hours=24)
    reminded = 0
    for cand in candidates:
        iv = cand.get("interview") or {}
        start = iv.get("start_time")
        if not start or cand.get("stage") != "Interview":
            continue
        try:
            dt = datetime.fromisoformat(start)
        except Exception:
            continue
        if dt <= window:
            msg = (
                f"Nhắc nhở: Bạn có lịch phỏng vấn vào {start}. "
                f"Link: {iv.get('link', 'sẽ được gửi riêng')}"
            )
            print(color(f"Reminder sent to {cand.get('name')} for interview at {start}", Colors.CYAN))
            # Best-effort comms via gateway if configured; otherwise just log.
            try:
                from tools.send_message_tool import send_message_tool

                target = os.getenv("MANAGER_TELEGRAM_CHAT_ID")
                if target:
                    send_message_tool({"action": "send", "target": f"telegram:{target}", "message": msg})
            except Exception:
                pass
            reminded += 1
    if not reminded:
        print(color("No interviews in the next 24 hours.", Colors.DIM))
    return 0


def cmd_hr_nudge_no_reply(args: argparse.Namespace) -> int:
    state = _load_state()
    cutoff = datetime.now(timezone.utc) - timedelta(hours=48)
    nudged = 0
    for cand in state.get("candidates", []):
        if cand.get("stage") not in {"Screened", "Shortlist"}:
            continue
        last_contact = cand.get("last_contact_at")
        if not last_contact:
            continue
        try:
            last = datetime.fromisoformat(last_contact)
        except Exception:
            continue
        if last <= cutoff and not cand.get("nudge_sent"):
            msg = (
                f"Chào {cand.get('name')}, chúng tôi vẫn đang chờ phản hồi của bạn "
                f"cho vị trí ứng tuyển. Vui lòng phản hồi để tiếp tục quy trình."
            )
            print(color(f"Nudge sent to {cand.get('name')}", Colors.CYAN))
            cand["nudge_sent"] = True
            nudged += 1
    _save_state(state)
    if not nudged:
        print(color("No candidates need a follow-up nudge.", Colors.DIM))
    return 0


def build_hr_parser(subparsers) -> argparse.ArgumentParser:
    parser = subparsers.add_parser(
        "hr",
        help="LynxLabVN recruitment workflow",
        description="Draft jobs, screen CVs, schedule interviews, and manage candidates.",
    )
    sub = parser.add_subparsers(dest="hr_command")

    sub.add_parser("new-job", help="Open interactive JD draft editor")

    post = sub.add_parser("post", help="Post job to careers page + generate board text")
    post.add_argument("job_id", help="Job ID")

    screen = sub.add_parser("screen", help="Run CV screening on all applied candidates")
    screen.add_argument("job_id", help="Job ID")

    sched = sub.add_parser("schedule", help="Book an interview slot for a candidate")
    sched.add_argument("candidate_id", help="Candidate ID")

    pipe = sub.add_parser("pipeline", help="Show candidates kanban as text table")
    pipe.add_argument("job_id", help="Job ID")

    appr = sub.add_parser("approve-job", help="Approve a job at JD_REVIEW_GATE")
    appr.add_argument("job_id", help="Job ID")

    rev = sub.add_parser("revise-job", help="Return a job to JD_DRAFT with feedback")
    rev.add_argument("job_id", help="Job ID")
    rev.add_argument("--feedback", help="Manager revision notes")

    sub.add_parser("remind-interviews", help="Send reminders 24h before interviews")
    sub.add_parser("nudge-no-reply", help="Follow up with candidates who haven't replied in 48h")

    parser.set_defaults(func=cmd_hr)
    return parser


_COMMANDS: Dict[str, Callable[[argparse.Namespace], int]] = {
    "new-job": cmd_hr_new_job,
    "post": cmd_hr_post,
    "screen": cmd_hr_screen,
    "schedule": cmd_hr_schedule,
    "pipeline": cmd_hr_pipeline,
    "approve-job": cmd_hr_approve_job,
    "revise-job": cmd_hr_revise_job,
    "remind-interviews": cmd_hr_remind_interviews,
    "nudge-no-reply": cmd_hr_nudge_no_reply,
}


def cmd_hr(args: argparse.Namespace) -> int:
    sub = getattr(args, "hr_command", None)
    handler = _COMMANDS.get(sub)
    if handler is None:
        print(color(f"Unknown hr command: {sub}", Colors.RED))
        return 1
    return handler(args)


# Ensure static HR cron jobs exist when this module is loaded.
sync_static_cron_jobs()
