"""`social publish <video> ...` — publish a local video via the web UI."""

import json
import sys

from ..browser import first_page, session
from ..dryrun import DryRunGate
from ..platforms import get_platform


def cmd_publish(args, log) -> int:
    platform = get_platform(args.platform)
    tags = [t.strip() for t in (args.tags or "").split(",") if t.strip()]
    dry_run = DryRunGate(enabled=args.dry_run)
    with session(args.platform, headed=args.headed, slowmo=args.slowmo,
                 channel=args.channel,
                 profile_dir_override=getattr(args, "profile_dir", None)) as ctx:
        page = first_page(ctx)
        result = platform.publish(
            page,
            video_path=args.video_path,
            caption=args.caption,
            privacy=args.privacy,
            tags=tags,
            thumbnail=args.thumbnail,
            schedule=args.schedule,
            dry_run=dry_run,
            log=log,
        )
    print(json.dumps(result, ensure_ascii=False, indent=2))
    return 0 if result.get("status") in ("published", "dry_run") else 1