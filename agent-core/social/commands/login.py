"""`social login <platform>` — headed manual login; persists profile."""

from ..browser import first_page, session
from ..platforms import get_platform


def cmd_login(args, log) -> int:
    platform = get_platform(args.platform)
    with session(args.platform, headed=True, slowmo=args.slowmo,
                 channel=args.channel,
                 profile_dir_override=getattr(args, "profile_dir", None)) as ctx:
        page = first_page(ctx)
        platform.do_login(page, log)
    return 0