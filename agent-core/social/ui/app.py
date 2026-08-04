"""Minimal FastAPI UI for the social tool. No build step (plain HTML + vanilla JS).

Shares the SAME platforms/* backend as the CLI. Localhost-only, no auth in v1
(documented). Publish/comments/reply run the synchronous Playwright flow in a
worker thread (asyncio.to_thread) so the event loop is not blocked. Login opens
a headed browser in a background thread and polls for the logged-in marker for
up to 5 minutes (no stdin needed).
"""

import asyncio
import logging
import secrets
import threading
import time
from pathlib import Path

from fastapi import FastAPI, File, Form, UploadFile
from fastapi.responses import HTMLResponse, JSONResponse
from fastapi.staticfiles import StaticFiles

from ..browser import first_page, session
from ..dryrun import DryRunGate
from ..platforms import get_platform, supported_platforms

STATIC_DIR = Path(__file__).parent / "static"
UPLOADS_DIR = Path.home() / ".hermes" / "social" / "uploads"
UPLOADS_DIR.mkdir(parents=True, exist_ok=True)

app = FastAPI(title="Social Tool")
app.mount("/static", StaticFiles(directory=str(STATIC_DIR)), name="static")

_log = logging.getLogger("social.ui")

# platform -> {"thread": Thread, "started": ts, "done": bool, "result": str}
_login_jobs: dict = {}
_LOGIN_TIMEOUT_S = 300


def _index() -> str:
    return (STATIC_DIR / "index.html").read_text(encoding="utf-8")


@app.get("/", response_class=HTMLResponse)
def index() -> str:
    return _index()


@app.get("/api/platforms")
def api_platforms() -> dict:
    return {"platforms": supported_platforms()}


# ---- login (background thread, no stdin) -------------------------------

def _login_worker(platform_name: str) -> None:
    job = _login_jobs[platform_name]
    try:
        platform = get_platform(platform_name)
        with session(platform_name, headed=True, slowmo=0) as ctx:
            page = first_page(ctx)
            page.goto(platform.home_url, wait_until="domcontentloaded", timeout=60_000)
            deadline = time.time() + _LOGIN_TIMEOUT_S
            while time.time() < deadline:
                try:
                    platform.login_check(page)
                    job["result"] = "logged in"
                    break
                except Exception:
                    page.wait_for_timeout(5000)
            else:
                job["result"] = "timed out waiting for login"
    except Exception as e:  # noqa: BLE001
        job["result"] = f"error: {e}"
    finally:
        job["done"] = True


@app.post("/api/login")
def api_login(platform: str = Form(...)) -> dict:
    if platform not in supported_platforms():
        return JSONResponse({"error": f"unsupported platform: {platform}"}, status_code=400)
    job = _login_jobs.get(platform)
    if job and not job.get("done") and job["thread"].is_alive():
        return {"status": "already_running", "platform": platform,
                "message": f"Browser already open for {platform} — log in within "
                           f"{_LOGIN_TIMEOUT_S}s, then close the window."}
    job = {"thread": None, "started": time.time(), "done": False, "result": ""}
    t = threading.Thread(target=_login_worker, args=(platform,), daemon=True)
    job["thread"] = t
    _login_jobs[platform] = job
    t.start()
    return {"status": "started", "platform": platform,
            "message": f"Browser opened on {platform}. Log in (incl. 2FA) within "
                       f"{_LOGIN_TIMEOUT_S}s, then close the window or wait."}


@app.get("/api/login/status")
def api_login_status(platform: str) -> dict:
    job = _login_jobs.get(platform)
    if not job:
        return {"platform": platform, "done": True, "result": "no login started"}
    return {"platform": platform, "done": job.get("done", False),
            "result": job.get("result", "")}


# ---- publish / comments / reply (worker thread) -------------------------

async def _to_thread(fn, *a, **kw):
    return await asyncio.to_thread(fn, *a, **kw)


@app.post("/api/publish")
async def api_publish(
    video: UploadFile = File(...),
    platform: str = Form("tiktok"),
    caption: str = Form(""),
    privacy: str = Form(""),
    tags: str = Form(""),
    dry_run: bool = Form(False),
):
    if platform not in supported_platforms():
        return JSONResponse({"error": f"unsupported platform: {platform}"}, status_code=400)
    ext = Path(video.filename or "video.mp4").suffix or ".mp4"
    tmp = UPLOADS_DIR / f"{secrets.token_hex(8)}{ext}"
    try:
        tmp.write_bytes(await video.read())
    except Exception as e:  # noqa: BLE001
        return JSONResponse({"error": f"could not save upload: {e}"}, status_code=500)

    tags_list = [t.strip() for t in tags.split(",") if t.strip()]

    def run() -> dict:
        platform_obj = get_platform(platform)
        with session(platform, headed=True, slowmo=0) as ctx:
            page = first_page(ctx)
            return platform_obj.publish(
                page, video_path=str(tmp), caption=caption,
                privacy=privacy or None, tags=tags_list, thumbnail=None,
                schedule=None, dry_run=DryRunGate(enabled=dry_run), log=_log,
            )

    try:
        result = await _to_thread(run)
        return result
    except Exception as e:  # noqa: BLE001
        return JSONResponse({"error": str(e)}, status_code=500)
    finally:
        try:
            tmp.unlink()
        except Exception:
            pass


@app.get("/api/comments")
async def api_comments(platform: str, video_url: str, max: int = 20):
    if platform not in supported_platforms():
        return JSONResponse({"error": f"unsupported platform: {platform}"}, status_code=400)

    def run() -> list:
        platform_obj = get_platform(platform)
        with session(platform, headed=True, slowmo=0) as ctx:
            page = first_page(ctx)
            return platform_obj.list_comments(page, video_url=video_url, max=max, log=_log)

    try:
        return {"platform": platform, "comments": await _to_thread(run)}
    except Exception as e:  # noqa: BLE001
        return JSONResponse({"error": str(e)}, status_code=500)


@app.post("/api/reply")
async def api_reply(platform: str = Form(...), comment_ref: str = Form(...),
                    text: str = Form(...), dry_run: bool = Form(False)):
    if platform not in supported_platforms():
        return JSONResponse({"error": f"unsupported platform: {platform}"}, status_code=400)

    def run() -> dict:
        platform_obj = get_platform(platform)
        with session(platform, headed=True, slowmo=0) as ctx:
            page = first_page(ctx)
            return platform_obj.reply(page, ref=comment_ref, text=text,
                                       dry_run=DryRunGate(enabled=dry_run), log=_log)

    try:
        return await _to_thread(run)
    except Exception as e:  # noqa: BLE001
        return JSONResponse({"error": str(e)}, status_code=500)