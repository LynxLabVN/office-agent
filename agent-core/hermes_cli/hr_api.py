"""
HR REST API for the LynxLabVN dashboard.

Endpoints mirror ``agent-core/web/src/lib/api.ts`` and persist state in the
same JSON files used by ``hermes hr``.
"""

from __future__ import annotations

import random
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Dict, List, Optional

from fastapi import APIRouter, File, HTTPException, Query, Request, UploadFile
from pydantic import BaseModel

from hermes_cli._domain_shared import (
    _new_id,
    _utcnow,
    find_item,
    load_domain_state,
    save_domain_state,
)
from hermes_cli.hr import CANDIDATE_STAGES, DOMAIN
from skills.audit import log_decision, log_pii_access, log_state_transition
from skills.hr import require_recruiter
from skills.hr.compliance import check_offer_compliance
from skills.hr.pii import get_vault
from skills.hr.roles import _get_user_role

router = APIRouter(prefix="/api/hr", tags=["hr"])

STATE_FILE = "recruitment.json"

DEFAULT_STATE: Dict[str, Any] = {
    "version": 1,
    "jobs": [],
    "candidates": [],
    "applications": [],
    "comms": [],
}


def _load_state() -> Dict[str, Any]:
    return load_domain_state(DOMAIN, STATE_FILE, DEFAULT_STATE)


def _save_state(state: Dict[str, Any]) -> None:
    save_domain_state(DOMAIN, STATE_FILE, state)


# ---------------------------------------------------------------------------
# Jobs
# ---------------------------------------------------------------------------


@router.get("/jobs")
async def list_jobs() -> Dict[str, List[Dict[str, Any]]]:
    state = _load_state()
    return {"jobs": state.get("jobs", [])}


@router.get("/jobs/{job_id}")
async def get_job(job_id: str) -> Dict[str, Any]:
    state = _load_state()
    job = find_item(state.get("jobs", []), job_id)
    if not job:
        raise HTTPException(status_code=404, detail="Job not found")
    return job


class CreateJobRequest(BaseModel):
    title: str
    department: str
    location: str
    status: str = "open"
    posted_at: Optional[str] = None
    description: Optional[str] = None


@router.post("/jobs")
async def create_job(body: CreateJobRequest) -> Dict[str, Any]:
    state = _load_state()
    job = {
        "id": _new_id("job-"),
        "title": body.title,
        "department": body.department,
        "location": body.location,
        "status": body.status,
        "posted_at": body.posted_at or _utcnow(),
    }
    state.setdefault("jobs", []).append(job)
    _save_state(state)
    return job


class UpdateJobRequest(BaseModel):
    title: Optional[str] = None
    department: Optional[str] = None
    location: Optional[str] = None
    status: Optional[str] = None
    description: Optional[str] = None


@router.put("/jobs/{job_id}")
async def update_job(job_id: str, body: UpdateJobRequest) -> Dict[str, Any]:
    state = _load_state()
    job = find_item(state.get("jobs", []), job_id)
    if not job:
        raise HTTPException(status_code=404, detail="Job not found")
    before = {k: job.get(k) for k in body.model_dump(exclude_unset=True).keys()}
    for key, value in body.model_dump(exclude_unset=True).items():
        if value is not None:
            job[key] = value
    _save_state(state)
    log_state_transition(
        actor="human:dashboard",
        target=job_id,
        before=before,
        after=body.model_dump(exclude_unset=True),
        meta={"domain": "hr", "endpoint": "jobs/update"},
    )
    return job


@router.delete("/jobs/{job_id}")
async def delete_job(job_id: str) -> Dict[str, bool]:
    state = _load_state()
    state["jobs"] = [j for j in state.get("jobs", []) if j.get("id") != job_id]
    _save_state(state)
    return {"ok": True}


class PostJobRequest(BaseModel):
    boards: List[str]


@router.post("/jobs/{job_id}/post")
async def post_job(job_id: str, body: PostJobRequest) -> Dict[str, bool]:
    state = _load_state()
    job = find_item(state.get("jobs", []), job_id)
    if not job:
        raise HTTPException(status_code=404, detail="Job not found")
    job["posted_boards"] = body.boards
    job["updated_at"] = _utcnow()
    _save_state(state)
    return {"ok": True}


# ---------------------------------------------------------------------------
# Pipeline
# ---------------------------------------------------------------------------


@router.get("/pipeline")
async def get_pipeline(job_id: Optional[str] = Query(None)) -> Dict[str, Any]:
    state = _load_state()
    applications = state.get("applications", [])
    if job_id:
        applications = [a for a in applications if a.get("job_id") == job_id]
    return {"applications": applications, "stages": CANDIDATE_STAGES}


class MoveApplicationRequest(BaseModel):
    application_id: str
    new_stage: str


@router.post("/pipeline/move")
async def move_application(body: MoveApplicationRequest) -> Dict[str, bool]:
    state = _load_state()
    application = find_item(state.get("applications", []), body.application_id)
    if not application:
        raise HTTPException(status_code=404, detail="Application not found")
    previous_stage = application.get("stage")
    application["stage"] = body.new_stage
    application["updated_at"] = _utcnow()
    _save_state(state)
    log_state_transition(
        actor="human:dashboard",
        target=body.application_id,
        before={"stage": previous_stage},
        after={"stage": body.new_stage},
        meta={"domain": "hr", "endpoint": "pipeline/move"},
    )
    return {"ok": True}


# ---------------------------------------------------------------------------
# Candidates
# ---------------------------------------------------------------------------


@router.get("/candidates")
async def list_candidates(request: Request) -> Dict[str, List[Dict[str, Any]]]:
    require_recruiter(request)
    state = _load_state()
    return {"candidates": state.get("candidates", [])}


@router.get("/candidates/{candidate_id}")
async def get_candidate(candidate_id: str, request: Request) -> Dict[str, Any]:
    require_recruiter(request)
    state = _load_state()
    candidate = find_item(state.get("candidates", []), candidate_id)
    if not candidate:
        raise HTTPException(status_code=404, detail="Candidate not found")
    log_pii_access(
        actor=f"human:{_get_user_role(request) or 'unknown'}",
        target=candidate_id,
        data_type="candidate_profile",
        meta={"endpoint": "candidates/{candidate_id}"},
    )
    return candidate


@router.get("/candidates/{candidate_id}/cv")
async def get_candidate_cv(candidate_id: str, request: Request) -> bytes:
    require_recruiter(request)
    candidate = await get_candidate(candidate_id)
    vault = get_vault()
    enc_path = candidate.get("cv_encrypted_path")
    if enc_path and Path(enc_path).exists():
        plaintext = vault.read_cv(Path(enc_path))
        log_pii_access(
            actor=f"human:{_get_user_role(request) or 'unknown'}",
            target=candidate_id,
            data_type="cv_file",
            meta={"endpoint": "candidates/{candidate_id}/cv", "encrypted": True},
        )
        return plaintext
    text = f"CV for {candidate.get('name', candidate_id)}\n\nSkills: TBD\nExperience: TBD\n"
    return text.encode("utf-8")


@router.post("/candidates/{candidate_id}/cv")
async def upload_candidate_cv(
    candidate_id: str,
    request: Request,
    file: UploadFile = File(...),
) -> Dict[str, Any]:
    """Upload and encrypt a candidate CV at rest."""
    require_recruiter(request)
    state = _load_state()
    candidate = find_item(state.get("candidates", []), candidate_id)
    if not candidate:
        raise HTTPException(status_code=404, detail="Candidate not found")
    contents = await file.read()
    vault = get_vault()
    enc_path = vault.store_cv(candidate_id, contents, file.filename or "cv.pdf")
    candidate["cv_encrypted_path"] = str(enc_path)
    candidate["updated_at"] = _utcnow()
    _save_state(state)
    log_pii_access(
        actor=f"human:{_get_user_role(request) or 'unknown'}",
        target=candidate_id,
        data_type="cv_upload",
        meta={"endpoint": "candidates/{candidate_id}/cv", "encrypted_path": str(enc_path)},
    )
    return {"ok": True, "encrypted_path": str(enc_path)}


class SetScoreOverrideRequest(BaseModel):
    application_id: str
    score_override: float
    reason: Optional[str] = None


@router.post("/applications/score-override")
async def set_score_override(body: SetScoreOverrideRequest) -> Dict[str, Any]:
    state = _load_state()
    application = find_item(state.get("applications", []), body.application_id)
    if not application:
        raise HTTPException(status_code=404, detail="Application not found")
    previous_score = application.get("cv_score")
    application["score_override"] = body.score_override
    application["score_override_reason"] = body.reason
    application["updated_at"] = _utcnow()
    _save_state(state)
    log_state_transition(
        actor="human:recruiter",
        target=body.application_id,
        before={"cv_score": previous_score},
        after={"score_override": body.score_override, "reason": body.reason},
        meta={"domain": "hr", "endpoint": "applications/score-override"},
    )
    return {
        "ok": True,
        "application_id": body.application_id,
        "previous_cv_score": previous_score,
        "score_override": body.score_override,
    }


class CompareCandidatesRequest(BaseModel):
    candidate_ids: List[str]
    job_id: str


@router.post("/compare")
async def compare_candidates(body: CompareCandidatesRequest) -> Dict[str, Any]:
    state = _load_state()
    candidates = [find_item(state.get("candidates", []), cid) for cid in body.candidate_ids]
    comparison = {
        str(i): {
            "candidate_id": c.get("id") if c else None,
            "name": c.get("name") if c else "Unknown",
            "score": c.get("score") if c else 0,
        }
        for i, c in enumerate(candidates)
    }
    return {"comparison": comparison}


class ScreenCandidatesRequest(BaseModel):
    job_id: str


@router.post("/screen")
async def screen_candidates(body: ScreenCandidatesRequest) -> Dict[str, bool]:
    state = _load_state()
    applications = [a for a in state.get("applications", []) if a.get("job_id") == body.job_id]
    for application in applications:
        application["stage"] = "Screened"
        application["updated_at"] = _utcnow()
    _save_state(state)
    return {"ok": True}


# ---------------------------------------------------------------------------
# Schedule
# ---------------------------------------------------------------------------


@router.get("/schedule/slots")
async def get_schedule_slots(event_type_id: str, date: str) -> Dict[str, List[Dict[str, Any]]]:
    slots = []
    for hour in range(9, 18):
        start = f"{date}T{hour:02d}:00:00+00:00"
        end = f"{date}T{hour:02d}:30:00+00:00"
        slots.append({"start": start, "end": end, "available": random.random() > 0.3})
        start = f"{date}T{hour:02d}:30:00+00:00"
        end = f"{date}T{(hour + 1):02d}:00:00+00:00"
        slots.append({"start": start, "end": end, "available": random.random() > 0.3})
    return {"slots": slots}


class BookScheduleRequest(BaseModel):
    event_type_id: str
    start: str
    candidate_id: str


@router.post("/schedule/book")
async def book_schedule(body: BookScheduleRequest) -> Dict[str, Any]:
    return {"ok": True, "booking_url": f"https://calendly.example.com/{body.candidate_id}/{body.start}"}


# ---------------------------------------------------------------------------
# Comms
# ---------------------------------------------------------------------------


@router.get("/comms/log")
async def get_comms_log(
    candidate_id: Optional[str] = Query(None),
    channel: Optional[str] = Query(None),
) -> Dict[str, List[Dict[str, Any]]]:
    state = _load_state()
    messages = state.get("comms", [])
    if candidate_id:
        messages = [m for m in messages if m.get("candidate_id") == candidate_id]
    if channel:
        messages = [m for m in messages if m.get("channel") == channel]
    return {"messages": messages}


class SendCommsRequest(BaseModel):
    candidate_id: str
    channel: str
    text: str
    template: Optional[str] = None


@router.post("/comms/send")
async def send_comms(body: SendCommsRequest) -> Dict[str, bool]:
    state = _load_state()
    state.setdefault("comms", []).append(
        {
            "id": _new_id("msg-"),
            "candidate_id": body.candidate_id,
            "channel": body.channel,
            "text": body.text,
            "sent_at": _utcnow(),
        }
    )
    _save_state(state)
    return {"ok": True}


# ---------------------------------------------------------------------------
# Analytics
# ---------------------------------------------------------------------------


@router.get("/analytics/overview")
async def analytics_overview(
    date_from: Optional[str] = Query(None),
    date_to: Optional[str] = Query(None),
) -> Dict[str, Any]:
    return {
        "date_from": date_from or "2026-01-01",
        "date_to": date_to or datetime.now(timezone.utc).strftime("%Y-%m-%d"),
        "time_to_hire_days": random.randint(18, 45),
        "funnel": {
            "Applied": 120,
            "Screened": 80,
            "Shortlist": 40,
            "Interview": 20,
            "Offer": 8,
            "Hired": 4,
        },
        "source_effectiveness": {
            "LinkedIn": 45,
            "Referral": 30,
            "Job Board": 20,
            "Agency": 5,
        },
    }


# ---------------------------------------------------------------------------
# Offer compliance
# ---------------------------------------------------------------------------


class CheckOfferComplianceRequest(BaseModel):
    offer_text: str


@router.post("/offer/check-compliance")
async def check_offer(body: CheckOfferComplianceRequest) -> Dict[str, Any]:
    result = check_offer_compliance(body.offer_text)
    log_decision(
        actor="agent",
        target="offer",
        decision="passed" if result["passed"] else "flagged",
        context={"missing_required": result.get("missing_required", [])},
    )
    return result
