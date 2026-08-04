"""Generate a branded Eir icon.ico (multi-size) for the Inno Setup installer."""
from pathlib import Path
from PIL import Image


ROOT = Path(__file__).resolve().parent.parent.parent
LOGO_PNG = ROOT / "agent-core" / "apps" / "bootstrap-installer" / "public" / "eir-logo.png"
OUT_ICO = Path("build/installer/icon.ico")


def load_square_logo(size: int) -> Image.Image:
    """Load the Eir stag logo and fit it into a centered square of the given size."""
    src = Image.open(LOGO_PNG).convert("RGBA")
    src_w, src_h = src.size
    canvas_size = max(src_w, src_h)
    canvas = Image.new("RGBA", (canvas_size, canvas_size), (0, 0, 0, 0))
    canvas.paste(src, ((canvas_size - src_w) // 2, (canvas_size - src_h) // 2), src)
    return canvas.resize((size, size), Image.Resampling.LANCZOS)


if __name__ == "__main__":
    sizes = [(256, 256), (128, 128), (64, 64), (48, 48), (32, 32), (16, 16)]
    frames = [load_square_logo(w) for w, h in sizes]
    OUT_ICO.parent.mkdir(parents=True, exist_ok=True)
    # Save multi-resolution ICO: first frame is the largest, append smaller sizes.
    frames[0].save(OUT_ICO, format="ICO", sizes=sizes, append_images=frames[1:])
    print(f"{OUT_ICO} written")
