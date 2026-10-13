"""Regenerate committed application icons. Requires Pillow."""

from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "傲然凝視的藍晶女戰士.png"
ASSETS = ROOT / "assets"


def main():
    ASSETS.mkdir(exist_ok=True)
    with Image.open(SOURCE) as source:
        icon = source.convert("RGBA")
        icon.thumbnail((256, 256), Image.Resampling.LANCZOS)
        canvas = Image.new("RGBA", (256, 256), (0, 0, 0, 0))
        canvas.paste(icon, ((256 - icon.width) // 2, (256 - icon.height) // 2))
        canvas.save(ASSETS / "icon.png")
        canvas.save(
            ASSETS / "icon.ico",
            sizes=[(size, size) for size in (16, 24, 32, 48, 64, 128, 256)],
        )
    print("Generated assets/icon.png and assets/icon.ico")


if __name__ == "__main__":
    main()
