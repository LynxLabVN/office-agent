"""
``hermes marketing`` subcommand — LynxLabVN marketing video pipeline.

Drives the 14-state pipeline from product selection through publish/analysis.
State is persisted in ``~/.hermes/marketing/pipeline.json``.
"""

from __future__ import annotations

import argparse
import json
import os
import sys
from pathlib import Path
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

DOMAIN = "marketing"
STATE_FILE = "pipeline.json"

# 14-state pipeline
STATES = [
    "PRODUCT_SELECT",
    "FORMAT_PICK",
    "SCRIPT_DRAFT",
    "HOOK_ITERATE",
    "MANAGER_REVIEW_GATE",
    "SHOOT_BRIEF",
    "FOOTAGE_INGEST",
    "AUTO_EDIT",
    "FINAL_REVIEW_GATE",
    "PUBLISH",
    "MONITOR",
    "REPLY_QUEUE",
    "ANALYZE",
    "LEDGER",
]

# Format picker: sku / category -> (recommended mix, reasoning)
FORMAT_PICKER: Dict[str, tuple[str, str]] = {
    "MA5": ("UGC + unboxing + BTS", "Visual-first AR glasses; social proof and BTS build trust."),
    "A14": ("UGC + unboxing + BTS", "Buyers want to see image quality and real-world mounting."),
    "AD35": ("UGC + unboxing + BTS", "Screen products need unboxing to show panel, build, and ports."),
    "A8": ("UGC + unboxing + BTS", "Demo-heavy; BTS shows stability and portability."),
    "GX200": ("UGC + unboxing + BTS", "Creators want to see the rig in a real setup."),
    "P011": ("UGC + unboxing + BTS", "Pocket visual accessory; UGC shows everyday carry use cases."),
    "bao đàn": ("demo + comparison + testimonial", "Function is protection/portability; comparison proves value."),
    "bao dan": ("demo + comparison + testimonial", "Function is protection/portability; comparison proves value."),
    "UHF": ("demo + comparison + testimonial", "Audio quality is invisible; demo + comparison make it audible."),
    "mic đeo tai": ("demo + comparison + testimonial", "Clarity and range matter; comparison against built-in mic is persuasive."),
    "mic deo tai": ("demo + comparison + testimonial", "Clarity and range matter; comparison against built-in mic is persuasive."),
}

AUDIO_SKUS = {"bao đàn", "bao dan", "UHF", "mic đeo tai", "mic deo tai"}
VISUAL_SKUS = {"MA5", "A14", "AD35", "A8", "GX200", "P011"}


def _load_state() -> Dict[str, Any]:
    return load_domain_state(DOMAIN, STATE_FILE)


def _save_state(state: Dict[str, Any]) -> None:
    save_domain_state(DOMAIN, STATE_FILE, state)


def _get_piece(state: Dict[str, Any], piece_id: str) -> Optional[Dict[str, Any]]:
    return find_item(state.get("items", []), piece_id)


def _category_for_sku(sku: str) -> str:
    if sku in AUDIO_SKUS:
        return "audio"
    if sku in VISUAL_SKUS:
        return "visual"
    return "new_launch"


def _fallback_product_specs(sku: str) -> Dict[str, Any]:
    category = _category_for_sku(sku)
    return {
        "sku": sku,
        "name": sku,
        "category": category,
        "description": f"Demo {category} product ({sku}) for pipeline testing.",
        "price_vnd": 0,
        "tags": [category, "lynxlab", sku.lower()],
    }


def _fetch_product_specs(sku: str) -> Dict[str, Any]:
    result = call_mcp_tool(
        "mcp-catalog",
        "get_product_specs",
        {"sku": sku},
        fallback={"result": _fallback_product_specs(sku)},
    )
    payload = result.get("result") if isinstance(result, dict) else result
    if isinstance(payload, str):
        try:
            payload = json.loads(payload)
        except Exception:
            pass
    if isinstance(payload, dict):
        return payload
    return _fallback_product_specs(sku)


def _pick_format(product: Dict[str, Any]) -> tuple[str, str]:
    sku = str(product.get("sku") or product.get("name") or "")
    if sku in FORMAT_PICKER:
        return FORMAT_PICKER[sku]
    category = product.get("category", "")
    if category == "audio":
        return "demo + comparison + testimonial", "Audio product: let the audience hear the difference."
    if category == "visual":
        return "UGC + unboxing + BTS", "Visual product: show real use, unboxing, and production value."
    return "short-form + storytelling", "New launch: maximize reach with fast, story-driven content."


def _generate_hooks(product: Dict[str, Any], fmt: str) -> List[str]:
    name = product.get("name") or product.get("sku", "Sản phẩm")
    return [
        f"Mở hộp {name} — chi tiết đầu tiên đã khiến tôi bất ngờ.",
        f"Tôi đã thử {name} trong 1 tuần, đây là sự thật.",
        f"Tại sao {name} lại đáng mua hơn bản rẻ tiền?",
    ]


def _generate_script(product: Dict[str, Any], fmt: str) -> Dict[str, Any]:
    name = product.get("name") or product.get("sku", "Sản phẩm")
    sku = product.get("sku", name)
    category = product.get("category", "visual")
    duration = 45 if category == "visual" else 60

    shotlist = [
        {
            "duration": 3,
            "purpose": "Hook",
            "dialogue": f"Mở hộp {name} ngay bây giờ.",
            "action": "Hands place product box on desk",
            "angle": "Top-down",
            "b_roll": "Box logo close-up",
            "props": ["box"],
            "on_screen_text": f"{sku} — trải nghiệm mới",
        },
        {
            "duration": 5,
            "purpose": "Unboxing",
            "dialogue": "Cùng xem bên trong có gì.",
            "action": "Open box and reveal layers",
            "angle": "Top-down",
            "b_roll": "Peel seal slowly",
            "props": ["box", "cutter"],
            "on_screen_text": "Unboxing",
        },
        {
            "duration": 7,
            "purpose": "Feature highlight",
            "dialogue": f"{name} nhỏ gọn đến khó tin.",
            "action": "Pick up product, rotate in hand",
            "angle": "45° side",
            "b_roll": "Close-up of details",
            "props": [name],
            "on_screen_text": "Thiết kế",
        },
        {
            "duration": 10,
            "purpose": "Demo",
            "dialogue": "Đeo vào và trải nghiệm ngay.",
            "action": "Wear / operate product",
            "angle": "POV + front",
            "b_roll": "Screen or action close-up",
            "props": [name, "phone"],
            "on_screen_text": "Demo",
        },
        {
            "duration": 8,
            "purpose": "Before/after or comparison",
            "dialogue": "Trước và sau khi dùng — khác biệt rõ rệt.",
            "action": "Split screen or side-by-side",
            "angle": "Split",
            "b_roll": "Old device/product shot",
            "props": [name, "old device"],
            "on_screen_text": "So sánh",
        },
        {
            "duration": 7,
            "purpose": "User reaction",
            "dialogue": "Thực sự mượt và dễ dùng.",
            "action": "Smile and nod",
            "angle": "Front cam",
            "b_roll": "Close-up face",
            "props": [],
            "on_screen_text": "Trải nghiệm",
        },
        {
            "duration": 5,
            "purpose": "CTA",
            "dialogue": "Link trong bio — ưu đãi tuần này.",
            "action": "Point to caption/link",
            "angle": "Front cam",
            "b_roll": "Product hero shot",
            "props": [name],
            "on_screen_text": "Mua ngay",
        },
    ]

    return {
        "overview": {
            "product": sku,
            "goal": "Generate awareness and conversions among Vietnamese tech/creator audience.",
            "audience": "Vietnamese creators and early adopters, 18-35, TikTok + Facebook Reels.",
            "platform": "TikTok, Facebook Reels",
            "duration": duration,
            "format": fmt,
        },
        "shoot_requirements": {
            "aspect_ratio": "9:16",
            "resolution_fps": "1080p60",
            "tone_mood": "Curious, energetic, premium-but-accessible",
        },
        "setting": {
            "location": "Clean desk near window",
            "time": "Late morning, natural light",
            "lighting": "Key light 45° camera left, window fill on the right",
            "rationale": "Shows product clearly without a sterile studio look.",
        },
        "props_wardrobe": {
            "props": [sku, "charging cable", "branded cleaning cloth", "phone"],
            "wardrobe": "Solid black T-shirt, no visible logos",
        },
        "timeline_shotlist": shotlist,
        "scenes": [
            "Hook — box reveal",
            "Product close-up",
            "Feature highlight",
            "Operation / how it works",
            "Demo / use case",
            "Before/after or comparison",
            "User reaction",
            "CTA",
        ],
        "text_on_screen": {
            "lines": [sku, "Trải nghiệm mới", "Made in VN", "Ưu đãi tuần này"],
            "keywords": [sku, "Made in VN", "trải nghiệm"],
            "price_offer": "Ưu đãi giới hạn trong tuần",
        },
        "shoot_notes": {
            "pitfalls": ["Keep background uncluttered", "Avoid fingerprints on lenses/screen"],
            "must_see_details": ["Charging port", "Build quality", "LED/status indicator"],
            "backup_shots": ["Static product hero", "Over-shoulder use shot"],
            "per_scene_time": {s["purpose"]: s["duration"] for s in shotlist},
            "priority": "Clarity of product value is top priority",
        },
        "pre_shoot_checklist": {
            "hook_under_3s": True,
            "problem_in_first_5s": True,
            "benefit_demonstrated": True,
            "demo_real_use": True,
            "cta_specific": True,
            "setting_locked": True,
            "executable": True,
        },
    }


def _render_script_summary(script: Dict[str, Any]) -> str:
    overview = script.get("overview", {})
    lines = [
        f"Format: {overview.get('format')}",
        f"Duration: {overview.get('duration')}s",
        f"Platforms: {overview.get('platform')}",
        f"Scenes: {', '.join(script.get('scenes', [])[:4])}...",
    ]
    return "\n".join(lines)


def _create_piece(sku: str) -> Dict[str, Any]:
    return {
        "id": _new_id("mkt-"),
        "sku": sku,
        "state": "PRODUCT_SELECT",
        "created_at": _utcnow(),
        "updated_at": _utcnow(),
        "history": [],
    }


def _transition(piece: Dict[str, Any], new_state: str, note: str = "") -> None:
    add_history(piece, new_state, note)
    piece["state"] = new_state


def _send_manager_gate(piece: Dict[str, Any]) -> None:
    summary = _render_script_summary(piece.get("script", {}))
    hook = piece.get("hook", "")
    full_summary = (
        f"Product: {piece.get('product_name', piece.get('sku'))}\n"
        f"SKU: {piece.get('sku')}\n"
        f"Format: {piece.get('format')}\n"
        f"Hook: {hook}\n\n"
        f"Script summary:\n{summary}"
    )
    send_manager_review(
        title="Marketing Review Required",
        summary=full_summary,
        approve_callback=f"hermes marketing approve {piece['id']}",
        revise_callback=f"hermes marketing revise {piece['id']}",
        domain=DOMAIN,
        item_id=piece["id"],
    )


def cmd_marketing_new(args: argparse.Namespace) -> int:
    sku = args.product_sku
    state = _load_state()
    piece = _create_piece(sku)
    state.setdefault("items", []).append(piece)

    # PRODUCT_SELECT
    print(color(f"Resolving product: {sku}", Colors.CYAN))
    product = _fetch_product_specs(sku)
    piece["product"] = product
    piece["product_name"] = product.get("name") or sku
    _transition(piece, "PRODUCT_SELECT", f"Fetched specs for {sku}")

    # FORMAT_PICK
    fmt, reason = _pick_format(product)
    piece["format"] = fmt
    piece["format_reason"] = reason
    _transition(piece, "FORMAT_PICK", f"Selected format: {fmt}")

    # SCRIPT_DRAFT
    print(color("Calling ledger for hook inspiration...", Colors.DIM))
    leaderboard = call_mcp_tool(
        "mcp-ledger",
        "get_hooks_leaderboard",
        {"category": product.get("category", "visual")},
        fallback={"result": []},
    )
    if not isinstance(leaderboard, dict) or "result" not in leaderboard:
        print_mcp_fallback_notice(DOMAIN, "mcp-ledger", "get_hooks_leaderboard")
    script = _generate_script(product, fmt)
    piece["script"] = script
    _transition(piece, "SCRIPT_DRAFT", "Generated 9-section script")

    # HOOK_ITERATE
    hooks = _generate_hooks(product, fmt)
    piece["hooks"] = hooks
    piece["hook"] = hooks[0]
    _transition(piece, "HOOK_ITERATE", f"Selected hook: {hooks[0]}")

    # MANAGER_REVIEW_GATE
    _transition(piece, "MANAGER_REVIEW_GATE", "Awaiting manager approval")
    _save_state(state)
    _send_manager_gate(piece)

    print()
    print(color("┌─────────────────────────────────────────────────────────────┐", Colors.CYAN))
    print(color("│         New marketing piece created                         │", Colors.CYAN))
    print(color("└─────────────────────────────────────────────────────────────┘", Colors.CYAN))
    print(f"  Piece ID:    {color(piece['id'], Colors.YELLOW)}")
    print(f"  Product:     {piece['product_name']} ({piece['sku']})")
    print(f"  Format:      {piece['format']}")
    print(f"  Hook:        {piece['hook']}")
    print(f"  State:       {color(piece['state'], Colors.GREEN)}")
    print()
    print(color("Manager review request sent to Telegram (if configured).", Colors.DIM))
    print(color("Approve:  ", Colors.DIM) + f"hermes marketing approve {piece['id']}")
    print(color("Revise:   ", Colors.DIM) + f"hermes marketing revise {piece['id']}")
    return 0


def cmd_marketing_review_queue(args: argparse.Namespace) -> int:
    state = _load_state()
    pieces = [p for p in state.get("items", []) if p.get("state") == "MANAGER_REVIEW_GATE"]
    if not pieces:
        print(color("No pieces waiting at MANAGER_REVIEW_GATE.", Colors.DIM))
        return 0
    print(color("Pieces waiting for manager review:", Colors.CYAN))
    for p in pieces:
        print(f"  {color(p['id'], Colors.YELLOW)} {p.get('product_name')} ({p.get('format')})")
    return 0


def cmd_marketing_publish(args: argparse.Namespace) -> int:
    piece_id = args.piece_id
    state = _load_state()
    piece = _get_piece(state, piece_id)
    if not piece:
        print(color(f"Piece not found: {piece_id}", Colors.RED))
        return 1

    print(color(f"Publishing piece {piece_id}...", Colors.CYAN))
    # Best-effort platform uploads
    platforms = ["youtube", "meta", "tiktok"]
    posts: Dict[str, str] = dict(piece.get("posts") or {})
    for platform in platforms:
        result = call_mcp_tool(
            f"mcp-social-{platform}",
            "upload",
            {"piece_id": piece_id, "title": piece.get("hook", "")},
            fallback={"result": f"{platform}: demo fallback (server not configured)"},
        )
        msg = result.get("result") if isinstance(result, dict) else str(result)
        print(f"  {platform}: {msg}")
        # Capture the published post id per platform so the reply sweep can
        # target it later. upload returns {"video_id": ...} (youtube/tiktok)
        # or {"media_id": ...} (meta).
        post_id = _extract_post_id(result)
        if post_id:
            posts[platform] = post_id
    if posts:
        piece["posts"] = posts

    ledger = call_mcp_tool(
        "mcp-ledger",
        "record_post",
        {
            "piece_id": piece_id,
            "sku": piece.get("sku"),
            "format": piece.get("format"),
            "hook": piece.get("hook"),
        },
        fallback={"result": "ledger: demo fallback"},
    )
    if not isinstance(ledger, dict) or "result" not in ledger:
        print_mcp_fallback_notice(DOMAIN, "mcp-ledger", "record_post")

    _transition(piece, "PUBLISH", "Published to enabled platforms and recorded in ledger")
    _save_state(state)
    print(color(f"Piece {piece_id} is now in state PUBLISH.", Colors.GREEN))
    return 0


def _unwrap_mcp_payload(result: Any) -> Any:
    """Extract the payload from a ``call_mcp_tool`` result.

    MCP tool results arrive as ``{"result": "<json string>"}`` (the tool's
    JSON-serialized output carried as text). Parse the string when present so
    callers get the native dict/list. Tolerant of already-parsed payloads and
    error/fallback shapes.
    """
    if isinstance(result, dict):
        if "error" in result and "result" not in result:
            return result
        payload = result.get("result", result)
    else:
        payload = result
    if isinstance(payload, str):
        text = payload.strip()
        if not text:
            return {}
        try:
            return json.loads(text)
        except (json.JSONDecodeError, ValueError):
            return {"text": text}
    return payload


def _extract_post_id(result: Any) -> str:
    """Pull the published post id (video_id / media_id) from an upload result."""
    payload = _unwrap_mcp_payload(result)
    if isinstance(payload, dict):
        return str(
            payload.get("video_id")
            or payload.get("media_id")
            or payload.get("publish_id")
            or ""
        )
    return ""


# Per-platform MCP reply tool name + reply-policy template mapping.
# tiktok uses reply_comment (added for parity); meta's tool is `reply`.
_REPLY_TOOL = {
    "youtube": "reply_comment",
    "meta": "reply",
    "tiktok": "reply_comment",
}
# reply-policy templates.toml platform/scenario keys per social MCP.
_REPLY_TEMPLATE = {
    "youtube": ("youtube", "youtube_thank_you"),
    "meta": ("facebook", "social_comment_dm"),
    "tiktok": ("tiktok", "tiktok_thank_you"),
}


def _load_reply_policy():
    """Load the reply-policy skill's ``policy`` module.

    The skill directory uses a hyphen (``reply-policy``) so it is not a normal
    importable package; load ``policy.py`` via importlib from the bundled
    skills dir instead.
    """
    import importlib.util
    from hermes_constants import get_bundled_skills_dir

    skills_dir = get_bundled_skills_dir(Path(__file__).parent.parent / "skills")
    policy_path = skills_dir / "reply-policy" / "policy.py"
    if not policy_path.is_file():
        raise FileNotFoundError(f"reply-policy policy.py not found at {policy_path}")
    spec = importlib.util.spec_from_file_location("reply_policy_policy", policy_path)
    mod = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(mod)
    return mod.decide_reply, mod.load_templates


def cmd_marketing_reply(args: argparse.Namespace) -> int:
    """List comments on a piece's published posts and reply per reply-policy.

    For each platform the piece was published to, pull comments, run each
    through ``skills/reply-policy`` ``decide_reply`` (mode from
    ``REPLY_POLICY_MODE``, default ``suggest`` = queue for human), and act on
    the decision: ``send`` -> reply (unless --dry-run), ``queue_human`` ->
    log, ``drop`` -> skip. Requires the piece to have been published (post ids
    are recorded on ``cmd_marketing_publish``).
    """
    piece_id = getattr(args, "piece_id", None)
    dry_run = bool(getattr(args, "dry_run", False))
    state = _load_state()
    if piece_id:
        piece = _get_piece(state, piece_id)
        if not piece:
            print(color(f"Piece not found: {piece_id}", Colors.RED))
            return 1
        pieces = [piece]
    else:
        # Sweep every piece that has published post ids.
        pieces = [p for p in state.get("items", []) if p.get("posts")]
        if not pieces:
            print(color(
                "No pieces with published posts. Run `hermes marketing publish <id>` "
                "first (post ids are recorded on publish).",
                Colors.DIM,
            ))
            return 0

    try:
        decide_reply, load_templates = _load_reply_policy()
    except Exception as exc:
        print(color(f"reply-policy skill unavailable: {exc}", Colors.RED))
        return 1

    mode = os.getenv("REPLY_POLICY_MODE", "suggest").lower()
    templates = load_templates()
    label = " [dry-run]" if dry_run else ""
    print(color(f"Reply sweep (mode={mode}){label}...", Colors.CYAN))

    for piece in pieces:
        posts: Dict[str, str] = dict(piece.get("posts") or {})
        if not posts:
            continue
        print(color(f"Piece {piece.get('id')}:", Colors.CYAN))
        for platform, video_id in posts.items():
            if platform not in _REPLY_TOOL:
                continue
            print(f"  {platform}: {len(video_id) and video_id[:24]} ...")
            lc = call_mcp_tool(
                f"mcp-social-{platform}",
                "list_comments",
                {"video_id": video_id, "max_results": 20},
                fallback={"result": "[]"},
            )
            comments = _unwrap_mcp_payload(lc)
            if not isinstance(comments, list):
                comments = []
            if not comments:
                print(color("    (no comments)", Colors.DIM))
                continue

            tpl_platform, scenario = _REPLY_TEMPLATE[platform]
            reply_tool = _REPLY_TOOL[platform]
            for c in comments:
                cid = str(c.get("comment_id") or c.get("id") or "")
                inbound = str(c.get("text") or c.get("content") or "")
                decision = decide_reply(
                    inbound,
                    tpl_platform,
                    scenario,
                    mode,
                    templates,
                    {"domain": "marketing"},
                )
                action = decision.get("action", "drop")
                reply_text = decision.get("reply") or ""
                reason = decision.get("reason", "")
                preview = reply_text[:40] + ("..." if len(reply_text) > 40 else "")
                if action == "send":
                    if dry_run:
                        print(color(
                            f"    [dry-run] would reply to {cid}: {preview}", Colors.YELLOW,
                        ))
                    else:
                        r = call_mcp_tool(
                            f"mcp-social-{platform}",
                            reply_tool,
                            {"comment_id": cid, "text": reply_text},
                            fallback={"result": ""},
                        )
                        if isinstance(r, dict) and r.get("error"):
                            print(color(
                                f"    reply to {cid} FAILED: {r['error']}", Colors.RED,
                            ))
                        else:
                            print(color(
                                f"    replied to {cid}: {preview}", Colors.GREEN,
                            ))
                elif action == "queue_human":
                    print(color(
                        f"    queued for human: {cid} ({reason}) reply={preview}", Colors.YELLOW,
                    ))
                else:
                    print(color(f"    dropped {cid}: {reason}", Colors.DIM))
    return 0


def cmd_marketing_status(args: argparse.Namespace) -> int:
    piece_id = args.piece_id
    state = _load_state()
    piece = _get_piece(state, piece_id)
    if not piece:
        print(color(f"Piece not found: {piece_id}", Colors.RED))
        return 1
    print()
    print(color("┌─────────────────────────────────────────────────────────────┐", Colors.CYAN))
    print(color("│         Marketing piece status                              │", Colors.CYAN))
    print(color("└─────────────────────────────────────────────────────────────┘", Colors.CYAN))
    print(f"  ID:      {color(piece['id'], Colors.YELLOW)}")
    print(f"  Product: {piece.get('product_name')} ({piece.get('sku')})")
    print(f"  State:   {color(piece['state'], Colors.GREEN)}")
    print(f"  Format:  {piece.get('format')}")
    print(f"  Hook:    {piece.get('hook')}")
    print("  History:")
    for h in piece.get("history", []):
        print(f"    - {h.get('timestamp')[:19]}  {h.get('state')}  {h.get('note', '')}")
    return 0


def cmd_marketing_approve(args: argparse.Namespace) -> int:
    piece_id = args.piece_id
    state = _load_state()
    piece = _get_piece(state, piece_id)
    if not piece:
        print(color(f"Piece not found: {piece_id}", Colors.RED))
        return 1
    if piece.get("state") != "MANAGER_REVIEW_GATE":
        print(color(f"Piece is not at MANAGER_REVIEW_GATE (current: {piece.get('state')})", Colors.YELLOW))
        return 1
    _transition(piece, "SHOOT_BRIEF", "Manager approved script")
    _save_state(state)
    print(color(f"Piece {piece_id} approved. State → SHOOT_BRIEF.", Colors.GREEN))
    print()
    print(color("Shoot brief:", Colors.CYAN))
    print(json.dumps(piece.get("script", {}), ensure_ascii=False, indent=2)[:1200])
    print(color("\n[Human crew must execute the shoot before FOOTAGE_INGEST]", Colors.DIM))
    return 0


def cmd_marketing_revise(args: argparse.Namespace) -> int:
    piece_id = args.piece_id
    feedback = args.feedback or "Manager requested revision."
    state = _load_state()
    piece = _get_piece(state, piece_id)
    if not piece:
        print(color(f"Piece not found: {piece_id}", Colors.RED))
        return 1
    if piece.get("state") != "MANAGER_REVIEW_GATE":
        print(color(f"Piece is not at MANAGER_REVIEW_GATE (current: {piece.get('state')})", Colors.YELLOW))
        return 1
    piece["manager_feedback"] = feedback
    _transition(piece, "SCRIPT_DRAFT", f"Revision requested: {feedback}")
    _save_state(state)
    print(color(f"Piece {piece_id} returned to SCRIPT_DRAFT with feedback:", Colors.YELLOW))
    print(f"  {feedback}")
    print(color(f"Re-run: hermes marketing approve {piece_id} after editing the script.", Colors.DIM))
    return 0


def cmd_marketing_pull_analytics(args: argparse.Namespace) -> int:
    print(color("Pulling analytics for last 24h...", Colors.CYAN))
    result = call_mcp_tool(
        "mcp-ledger",
        "get_performance",
        {"window": "24h"},
        fallback={"result": []},
    )
    data = result.get("result") if isinstance(result, dict) else result
    print(json.dumps(data, ensure_ascii=False, indent=2)[:2000])
    return 0


def cmd_marketing_report(args: argparse.Namespace) -> int:
    print(color("Generating weekly performance report...", Colors.CYAN))
    result = call_mcp_tool(
        "mcp-ledger",
        "query_what_worked",
        {"window": "7d"},
        fallback={"result": "No ledger data available (demo fallback)."},
    )
    summary = result.get("result") if isinstance(result, dict) else str(result)
    print(summary)
    chat_id = os.getenv("MANAGER_TELEGRAM_CHAT_ID")
    if chat_id:
        send_manager_review(
            title="Weekly Marketing Report",
            summary=str(summary)[:800],
            approve_callback="(no action required)",
            revise_callback="(no action required)",
            domain=DOMAIN,
            item_id="weekly-report",
        )
        print(color("Report sent to Telegram manager.", Colors.GREEN))
    return 0


def cmd_marketing_publish_queue(args: argparse.Namespace) -> int:
    print(color("Checking scheduled publishing queue...", Colors.CYAN))
    state = _load_state()
    now = _utcnow()
    due = [
        p for p in state.get("items", [])
        if p.get("state") == "FINAL_REVIEW_GATE" and p.get("scheduled_at") and p.get("scheduled_at") <= now
    ]
    if not due:
        print(color("No scheduled pieces are due.", Colors.DIM))
        return 0
    for piece in due:
        print(f"  Publishing {piece['id']} ...")
        # Re-use publish logic
        fake_args = argparse.Namespace(piece_id=piece["id"])
        cmd_marketing_publish(fake_args)
    return 0


def build_marketing_parser(subparsers) -> argparse.ArgumentParser:
    parser = subparsers.add_parser(
        "marketing",
        help="LynxLabVN marketing video pipeline",
        description="Create, review, publish, and analyze marketing video pieces.",
    )
    sub = parser.add_subparsers(dest="marketing_command")

    new = sub.add_parser("new", help="Start a new pipeline piece for a product SKU")
    new.add_argument("product_sku", help="Product SKU, e.g. MA5")

    sub.add_parser("review-queue", help="List pieces at MANAGER_REVIEW_GATE")

    pub = sub.add_parser("publish", help="Execute PUBLISH for a piece")
    pub.add_argument("piece_id", help="Piece ID")

    st = sub.add_parser("status", help="Show current state and history for a piece")
    st.add_argument("piece_id", help="Piece ID")

    appr = sub.add_parser("approve", help="Approve a piece at MANAGER_REVIEW_GATE")
    appr.add_argument("piece_id", help="Piece ID")

    rev = sub.add_parser("revise", help="Return a piece to SCRIPT_DRAFT with feedback")
    rev.add_argument("piece_id", help="Piece ID")
    rev.add_argument("--feedback", help="Manager revision notes")

    sub.add_parser("pull-analytics", help="Pull stats for posts in the last 24h")
    sub.add_parser("report", help="Generate weekly performance summary")
    sub.add_parser("publish-queue", help="Publish any pieces with scheduled_at <= now")

    rep = sub.add_parser("reply", help="List comments on a piece's posts and reply per reply-policy")
    rep.add_argument(
        "piece_id",
        nargs="?",
        help="Piece ID (must have been published). Omit to sweep all pieces with posts.",
    )
    rep.add_argument(
        "--dry-run",
        action="store_true",
        help="Print reply-policy decisions without sending any replies",
    )

    parser.set_defaults(func=cmd_marketing)
    return parser


_COMMANDS: Dict[str, Callable[[argparse.Namespace], int]] = {
    "new": cmd_marketing_new,
    "review-queue": cmd_marketing_review_queue,
    "publish": cmd_marketing_publish,
    "status": cmd_marketing_status,
    "approve": cmd_marketing_approve,
    "revise": cmd_marketing_revise,
    "pull-analytics": cmd_marketing_pull_analytics,
    "report": cmd_marketing_report,
    "publish-queue": cmd_marketing_publish_queue,
    "reply": cmd_marketing_reply,
}


def cmd_marketing(args: argparse.Namespace) -> int:
    sub = getattr(args, "marketing_command", None)
    handler = _COMMANDS.get(sub)
    if handler is None:
        print(color(f"Unknown marketing command: {sub}", Colors.RED))
        return 1
    return handler(args)


# Ensure static marketing cron jobs exist when this module is loaded.
sync_static_cron_jobs()
