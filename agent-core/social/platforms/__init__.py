"""Platform factory.

v1 ships TikTok only. FacebookPlatform exists as a v2 stub (see facebook.py)
but is deliberately NOT registered, so argparse/UI cannot select it and it
fails fast with PlatformNotSupported (no browser launched) if requested.
"""

from ..errors import PlatformNotSupported
from .tiktok import TikTokPlatform

# v1: tiktok only. Add "facebook": FacebookPlatform once v2 is implemented.
_REGISTRY = {
    "tiktok": TikTokPlatform,
}


def supported_platforms() -> list:
    return sorted(_REGISTRY)


def get_platform(name: str) -> "object":
    cls = _REGISTRY.get(name)
    if cls is None:
        raise PlatformNotSupported(
            f"{name!r} not supported. v1 supports: tiktok (facebook is v2)."
        )
    return cls()