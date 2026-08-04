"""`social reply <platform> <comment_ref> <text>` — reply to one comment."""

import json

from ..browser import first_page, session
from ..dryrun import DryRunGate
from ..platforms import get_platform


def cmd_reply(args, log) -> int:
    platform = get_platform(args.platform)
    dry_run = DryRunGate(enabled=args.dry_run)
    with session(args.platform, headed=args.headed, slowmo=args.slowmo,
                 channel=args.channel,
                 profile_dir_override=getattr(args, "profile_dir", None)) as ctx:
        page = first_page(ctx)
        result = platform.reply(page, ref=args.comment_ref, text=args.text,
                                 dry_run=dry_run, log=log)
    print(json.dumps(result, ensure_ascii=False, indent=2))
    return 0 if result.get("status") in ("replied", "dry_run") else 1