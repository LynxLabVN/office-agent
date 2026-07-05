"""
Marketing REST API for the LynxLabVN dashboard.

Endpoints mirror ``agent-core/web/src/lib/api.ts`` and persist state in the
same JSON files used by ``hermes marketing``.
"""

from __future__ import annotations

import random
from datetime import datetime, timezone
from typing import Any, Dict, List, Optional

from fastapi import APIRouter, HTTPException, Query
from pydantic import BaseModel, Field

from hermes_cli._domain_shared import (
    _new_id,
    _utcnow,
    find_item,
    load_domain_state,
    save_domain_state,
)
from hermes_cli.marketing import DOMAIN, STATES
from skills.audit import log_state_transition

router = APIRouter(prefix="/api/marketing", tags=["marketing"])

STATE_FILE = "pipeline.json"
CATALOG_FILE = "catalog.json"

DEFAULT_CATALOG: List[Dict[str, Any]] = [
    {"sku": "MA5", "name": "MA5 AR Glasses", "description": "Lightweight AR glasses", "price": 299.0},
    {"sku": "A14", "name": "A14 Action Cam", "description": "4K action camera", "price": 199.0},
    {"sku": "AD35", "name": "AD35 Monitor", "description": "35-inch curved display", "price": 449.0},
    {"sku": "A8", "name": "A8 Gimbal", "description": "Portable phone gimbal", "price": 129.0},
    {"sku": "GX200", "name": "GX200 Rig", "description": "Creator rig", "price": 599.0},
    {"sku": "P011", "name": "P011 Pocket Kit", "description": "Everyday carry accessory", "price": 79.0},
    {"sku": "bao đàn", "name": "Bao đàn protective case", "description": "Instrument protection", "price": 49.0},
    {"sku": "UHF", "name": "UHF Wireless Mic", "description": "Wireless microphone", "price": 159.0},
    {"sku": "mic đeo tai", "name": "Mic đeo tai", "description": "Ear-mounted microphone", "price": 89.0},
]


def _load_pipeline() -> Dict[str, Any]:
    return load_domain_state(DOMAIN, STATE_FILE, {"version": 1, "items": []})


def _save_pipeline(state: Dict[str, Any]) -> None:
    save_domain_state(DOMAIN, STATE_FILE, state)


def _load_catalog() -> Dict[str, Any]:
    return load_domain_state(DOMAIN, CATALOG_FILE, {"version": 1, "products": DEFAULT_CATALOG})


def _save_catalog(state: Dict[str, Any]) -> None:
    save_domain_state(DOMAIN, CATALOG_FILE, state)


def _piece_to_summary(piece: Dict[str, Any]) -> Dict[str, Any]:
    return {
        "id": piece["id"],
        "title": piece.get("product_name") or piece.get("sku") or piece["id"],
        "format": piece.get("format", ""),
        "owner": "marketing",
        "state": piece.get("state", "PRODUCT_SELECT"),
        "due": piece.get("due"),
        "thumbnail": piece.get("thumbnail"),
    }


# ---------------------------------------------------------------------------
# Pipeline
# ---------------------------------------------------------------------------


@router.get("/pipeline/items")
async def list_pipeline_items(state: Optional[str] = Query(None)) -> Dict[str, Any]:
    pipeline = _load_pipeline()
    items = pipeline.get("items", [])
    if state:
        items = [i for i in items if i.get("state") == state]
    return {"items": [_piece_to_summary(i) for i in items], "states": STATES}


class MovePieceRequest(BaseModel):
    piece_id: str
    new_state: str


@router.post("/pipeline/move")
async def move_piece(body: MovePieceRequest) -> Dict[str, bool]:
    pipeline = _load_pipeline()
    piece = find_item(pipeline.get("items", []), body.piece_id)
    if not piece:
        raise HTTPException(status_code=404, detail="Piece not found")
    previous_state = piece.get("state")
    piece["state"] = body.new_state
    piece.setdefault("history", []).append(
        {"state": body.new_state, "timestamp": _utcnow(), "note": "Moved from dashboard"}
    )
    piece["updated_at"] = _utcnow()
    _save_pipeline(pipeline)
    log_state_transition(
        actor="human:dashboard",
        target=body.piece_id,
        before={"state": previous_state},
        after={"state": body.new_state},
        meta={"domain": "marketing", "endpoint": "pipeline/move"},
    )
    return {"ok": True}


# ---------------------------------------------------------------------------
# Script
# ---------------------------------------------------------------------------


@router.get("/script")
async def get_script(piece_id: str) -> Dict[str, Any]:
    pipeline = _load_pipeline()
    piece = find_item(pipeline.get("items", []), piece_id)
    if not piece:
        raise HTTPException(status_code=404, detail="Piece not found")
    script = piece.get("script") or {"sections": {}, "hooks": []}
    return {
        "piece_id": piece_id,
        "sections": script.get("sections", {}),
        "hooks": piece.get("hooks", script.get("hooks", [])),
    }


class SaveScriptRequest(BaseModel):
    piece_id: str
    script_json: Dict[str, Any]


@router.post("/script/save")
async def save_script(body: SaveScriptRequest) -> Dict[str, bool]:
    pipeline = _load_pipeline()
    piece = find_item(pipeline.get("items", []), body.piece_id)
    if not piece:
        raise HTTPException(status_code=404, detail="Piece not found")
    piece["script"] = body.script_json
    piece["updated_at"] = _utcnow()
    _save_pipeline(pipeline)
    return {"ok": True}


class GenerateScriptRequest(BaseModel):
    product_sku: str
    format: str


def _default_script(product_sku: str, fmt: str) -> Dict[str, Any]:
    return {
        "hook": f"Stop scrolling — {product_sku} changes everything.",
        "problem": "Creators waste hours on complicated setups.",
        "solution": f"{product_sku} is built for one-touch results.",
        "proof": "10,000+ sold in the first month.",
        "offer": "Order today and get free shipping.",
        "urgency": "Limited launch bundle ends tonight.",
        "cta": "Tap the link in bio to claim yours.",
        "b_roll": "Unboxing, close-up detail, real-world use.",
        "captions": "Bold subtitles on every hook beat.",
        "format": fmt,
    }


@router.post("/script/generate")
async def generate_script(body: GenerateScriptRequest) -> Dict[str, Any]:
    catalog = _load_catalog()
    product = next(
        (p for p in catalog.get("products", []) if p.get("sku") == body.product_sku),
        {"sku": body.product_sku},
    )
    return {
        "piece_id": "",
        "sections": _default_script(body.product_sku, body.format),
        "hooks": [f"{body.product_sku} hook #{i}" for i in range(1, 4)],
    }


class SuggestHooksRequest(BaseModel):
    product_sku: str
    format: str


@router.post("/hooks/suggest")
async def suggest_hooks(body: SuggestHooksRequest) -> Dict[str, List[str]]:
    hooks = [
        f"Why I switched to {body.product_sku}",
        f"{body.product_sku} vs the competition",
        f"3 things nobody tells you about {body.product_sku}",
        f"This {body.format} made me delete my old gear",
        f"Stop overthinking — {body.product_sku} just works",
    ]
    return {"hooks": hooks}


# ---------------------------------------------------------------------------
# Review
# ---------------------------------------------------------------------------


@router.get("/review/pending")
async def pending_reviews() -> Dict[str, List[Dict[str, Any]]]:
    pipeline = _load_pipeline()
    items = [
        {
            "id": i["id"],
            "piece_id": i["id"],
            "title": i.get("product_name") or i.get("sku") or i["id"],
            "submitted_by": "marketing",
            "submitted_at": i.get("updated_at") or i.get("created_at") or _utcnow(),
        }
        for i in pipeline.get("items", [])
        if i.get("state") in {"MANAGER_REVIEW_GATE", "FINAL_REVIEW_GATE"}
    ]
    return {"items": items}


class ReviewDecisionRequest(BaseModel):
    piece_id: str
    decision: str  # approve | revise
    feedback: Optional[str] = None


@router.post("/review/decide")
async def review_decide(body: ReviewDecisionRequest) -> Dict[str, bool]:
    pipeline = _load_pipeline()
    piece = find_item(pipeline.get("items", []), body.piece_id)
    if not piece:
        raise HTTPException(status_code=404, detail="Piece not found")
    previous_state = piece.get("state")
    new_state = "SHOOT_BRIEF" if body.decision == "approve" else "SCRIPT_DRAFT"
    piece["state"] = new_state
    piece.setdefault("history", []).append(
        {"state": new_state, "timestamp": _utcnow(), "note": body.feedback or body.decision}
    )
    piece["updated_at"] = _utcnow()
    _save_pipeline(pipeline)
    log_state_transition(
        actor="human:dashboard",
        target=body.piece_id,
        before={"state": previous_state},
        after={"state": new_state},
        meta={"domain": "marketing", "decision": body.decision, "feedback": body.feedback},
    )
    return {"ok": True}


# ---------------------------------------------------------------------------
# Edit / preview
# ---------------------------------------------------------------------------


class IngestEditRequest(BaseModel):
    piece_id: str
    footage_path: str


@router.post("/edit/ingest")
async def ingest_edit(body: IngestEditRequest) -> Dict[str, bool]:
    pipeline = _load_pipeline()
    piece = find_item(pipeline.get("items", []), body.piece_id)
    if not piece:
        raise HTTPException(status_code=404, detail="Piece not found")
    previous_state = piece.get("state")
    piece["footage_path"] = body.footage_path
    piece["state"] = "AUTO_EDIT"
    piece["updated_at"] = _utcnow()
    _save_pipeline(pipeline)
    log_state_transition(
        actor="agent",
        target=body.piece_id,
        before={"state": previous_state},
        after={"state": "AUTO_EDIT", "footage_path": body.footage_path},
        meta={"domain": "marketing", "endpoint": "edit/ingest"},
    )
    return {"ok": True}


class RenderEditRequest(BaseModel):
    piece_id: str
    shotlist: List[str]
    captions: str
    music: Optional[str] = None


@router.post("/edit/render")
async def render_edit(body: RenderEditRequest) -> Dict[str, Any]:
    pipeline = _load_pipeline()
    piece = find_item(pipeline.get("items", []), body.piece_id)
    if not piece:
        raise HTTPException(status_code=404, detail="Piece not found")
    previous_state = piece.get("state")
    piece["preview_url"] = f"/api/marketing/edit/preview?piece_id={body.piece_id}"
    piece["state"] = "FINAL_REVIEW_GATE"
    piece["updated_at"] = _utcnow()
    _save_pipeline(pipeline)
    log_state_transition(
        actor="agent",
        target=body.piece_id,
        before={"state": previous_state},
        after={"state": "FINAL_REVIEW_GATE"},
        meta={"domain": "marketing", "endpoint": "edit/render"},
    )
    return {"ok": True, "preview_url": piece["preview_url"]}


@router.get("/edit/preview")
async def preview_edit(piece_id: str) -> bytes:
    # Return a tiny 1x1 transparent PNG placeholder.
    return bytes.fromhex(
        "89504e470d0a1a0a0000000d49484452"
        "00000001000000010802000000907753"
        "de0000000c4944415408d763f8ffff3f"
        "0005fe02fedcc98e130000000049454e"
        "44ae426082"
    )


# ---------------------------------------------------------------------------
# Publish
# ---------------------------------------------------------------------------


class SchedulePublishRequest(BaseModel):
    piece_id: str
    platforms: List[str]
    scheduled_at: str


@router.post("/publish/schedule")
async def schedule_publish(body: SchedulePublishRequest) -> Dict[str, bool]:
    pipeline = _load_pipeline()
    piece = find_item(pipeline.get("items", []), body.piece_id)
    if not piece:
        raise HTTPException(status_code=404, detail="Piece not found")
    previous_state = piece.get("state")
    piece["publish_schedule"] = {
        "platforms": body.platforms,
        "scheduled_at": body.scheduled_at,
    }
    piece["state"] = "PUBLISH"
    piece["updated_at"] = _utcnow()
    _save_pipeline(pipeline)
    log_state_transition(
        actor="agent",
        target=body.piece_id,
        before={"state": previous_state},
        after={"state": "PUBLISH"},
        meta={"domain": "marketing", "endpoint": "publish/schedule"},
    )
    return {"ok": True}


class PublishNowRequest(BaseModel):
    piece_id: str
    platforms: List[str]


@router.post("/publish/now")
async def publish_now(body: PublishNowRequest) -> Dict[str, Any]:
    pipeline = _load_pipeline()
    piece = find_item(pipeline.get("items", []), body.piece_id)
    if not piece:
        raise HTTPException(status_code=404, detail="Piece not found")
    previous_state = piece.get("state")
    piece["state"] = "MONITOR"
    piece["published_at"] = _utcnow()
    piece["updated_at"] = _utcnow()
    _save_pipeline(pipeline)
    log_state_transition(
        actor="agent",
        target=body.piece_id,
        before={"state": previous_state},
        after={"state": "MONITOR"},
        meta={"domain": "marketing", "endpoint": "publish/now", "platforms": body.platforms},
    )
    urls = {p: f"https://example.com/{p}/{piece['id']}" for p in body.platforms}
    return {"ok": True, "urls": urls}


# ---------------------------------------------------------------------------
# Comments
# ---------------------------------------------------------------------------


@router.get("/comments/list")
async def list_comments(platform: Optional[str] = Query(None)) -> Dict[str, List[Dict[str, Any]]]:
    pipeline = _load_pipeline()
    comments = pipeline.get("comments", [])
    if platform:
        comments = [c for c in comments if c.get("platform") == platform]
    return {"comments": comments}


class ReplyCommentRequest(BaseModel):
    comment_id: str
    platform: str
    text: str


@router.post("/comments/reply")
async def reply_to_comment(body: ReplyCommentRequest) -> Dict[str, bool]:
    pipeline = _load_pipeline()
    comments = pipeline.setdefault("comments", [])
    comment = find_item(comments, body.comment_id)
    if not comment:
        raise HTTPException(status_code=404, detail="Comment not found")
    comment["reply"] = {"text": body.text, "sent_at": _utcnow()}
    _save_pipeline(pipeline)
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
        "views": random.randint(8000, 45000),
        "likes": random.randint(400, 2500),
        "comments": random.randint(80, 600),
        "shares": random.randint(50, 400),
    }


@router.get("/analytics/drilldown")
async def analytics_drilldown(
    group_by: str = Query(...),
    product: Optional[str] = Query(None),
    format: Optional[str] = Query(None),
) -> Dict[str, List[Dict[str, Any]]]:
    rows = [
        {group_by: "Group A", "views": random.randint(1000, 9000), "likes": random.randint(50, 500)},
        {group_by: "Group B", "views": random.randint(1000, 9000), "likes": random.randint(50, 500)},
    ]
    return {"rows": rows}


@router.get("/analytics/hooks")
async def hooks_leaderboard() -> Dict[str, List[Dict[str, Any]]]:
    return {
        "leaderboard": [
            {"hook": "Why I switched to…", "uses": 42, "avg_views": 12000},
            {"hook": "Stop scrolling…", "uses": 38, "avg_views": 9500},
            {"hook": "3 things nobody tells you…", "uses": 31, "avg_views": 8700},
        ]
    }


# ---------------------------------------------------------------------------
# Catalog
# ---------------------------------------------------------------------------


@router.get("/catalog")
async def get_catalog() -> Dict[str, List[Dict[str, Any]]]:
    catalog = _load_catalog()
    return {"products": catalog.get("products", [])}


class CreateProductRequest(BaseModel):
    sku: Optional[str] = None
    name: str
    description: str
    price: Optional[float] = None
    image_url: Optional[str] = None


@router.post("/catalog")
async def create_product(body: CreateProductRequest) -> Dict[str, Any]:
    catalog = _load_catalog()
    sku = body.sku or _new_id("sku-")
    if any(p.get("sku") == sku for p in catalog.get("products", [])):
        raise HTTPException(status_code=409, detail="SKU already exists")
    product = {
        "sku": sku,
        "name": body.name,
        "description": body.description,
        "price": body.price,
        "image_url": body.image_url,
    }
    catalog.setdefault("products", []).append(product)
    _save_catalog(catalog)
    return product


class UpdateProductRequest(BaseModel):
    name: Optional[str] = None
    description: Optional[str] = None
    price: Optional[float] = None
    image_url: Optional[str] = None


@router.put("/catalog/{sku}")
async def update_product(sku: str, body: UpdateProductRequest) -> Dict[str, Any]:
    catalog = _load_catalog()
    product = next((p for p in catalog.get("products", []) if p.get("sku") == sku), None)
    if not product:
        raise HTTPException(status_code=404, detail="Product not found")
    for key, value in body.model_dump(exclude_unset=True).items():
        if value is not None:
            product[key] = value
    _save_catalog(catalog)
    return product


@router.delete("/catalog/{sku}")
async def delete_product(sku: str) -> Dict[str, bool]:
    catalog = _load_catalog()
    products = catalog.get("products", [])
    catalog["products"] = [p for p in products if p.get("sku") != sku]
    _save_catalog(catalog)
    return {"ok": True}
