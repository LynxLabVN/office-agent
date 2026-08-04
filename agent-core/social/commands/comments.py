"""`social comments <platform> <video_url> ...` — list comments on a video."""

import json
import sys

from ..browser import first_page, session
from ..platforms import get_platform


def cmd_comments(args, log) -> int:
    platform = get_platform(args.platform)
    with session(args.platform, headed=args.headed, slowmo=args.slowmo,
                 channel=args.channel,
                 profile_dir_override=getattr(args, "profile_dir", None)) as ctx:
        page = first_page(ctx)
        comments = platform.list_comments(
            page, video_url=args.video_url, max=args.max, log=log
        )
    if args.json:
        print(json.dumps(comments, ensure_ascii=False, indent=2))
        return 0
    if not comments:
        print("(no comments found)")
        return 0
    # human-readable table
    print(f"{'ref':<10} {'author':<22} {'likes':<7} text")
    print("-" * 70)
    for c in comments:
        text = c.get("text", "").replace("\n", " ")
        if len(text) > 40:
            text = text[:37] + "..."
        print(f"{c.get('comment_ref',''):<10} {(c.get('author','') or '')[:21]:<22} "
              f"{(c.get('likes','') or '')[:6]:<7} {text}")
    print(f"\n{len(comments)} comment(s). Refs expire — run `reply` immediately.")
    return 0