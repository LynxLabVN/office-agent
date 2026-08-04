"""`social ui` — launch the local no-build web UI (FastAPI + uvicorn)."""

from ..ui import run_ui


def cmd_ui(args, log) -> int:
    run_ui(host=args.host, port=args.port, log=log)
    return 0