"""Launch the local no-build web UI (FastAPI + uvicorn)."""

import logging


def run_ui(host: str = "127.0.0.1", port: int = 8000, log: logging.Logger | None = None):
    import uvicorn

    from .app import app

    if log:
        log.info("UI starting: http://%s:%d/ (localhost-only, no auth)", host, port)
    print(f"\nSocial UI: http://{host}:{port}/\n(localhost-only — do not expose)\n")
    uvicorn.run(app, host=host, port=port, log_level="info")