"""Generate a branded Eir icon.ico (multi-size) for the Inno Setup installer."""
from PIL import Image, ImageDraw, ImageFont


def make(size: int) -> Image.Image:
    img = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    r = max(2, size // 6)
    d.rounded_rectangle([0, 0, size - 1, size - 1], radius=r, fill=(10, 10, 10, 255))
    d.rounded_rectangle(
        [size * 0.04, size * 0.04, size * 0.96, size * 0.96],
        radius=max(1, r - 2),
        outline=(45, 212, 191, 255),
        width=max(1, size // 64),
    )
    try:
        font = ImageFont.truetype("arial.ttf", int(size * 0.62))
    except Exception:
        font = ImageFont.load_default()
    d.text((size * 0.30, size * 0.16), "E", font=font, fill=(45, 212, 191, 255))
    return img


if __name__ == "__main__":
    make(256).save(
        "build/installer/icon.ico",
        sizes=[(256, 256), (128, 128), (64, 64), (48, 48), (32, 32), (16, 16)],
    )
    print("icon.ico written")