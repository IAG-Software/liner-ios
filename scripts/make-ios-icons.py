"""Generate the iOS icon set from the desktop app icon.

iOS accepts "loose" PNG icons referenced from Info.plist (CFBundleIcons), which
avoids Xcode's `actool` asset-catalogue compiler entirely — a hard requirement
for this cross-compile pipeline.
"""

from pathlib import Path

from PIL import Image

SRC = Path(r"D:\ios2\upstream\build\icon-mac.png")
OUT = Path(r"D:\ios2\liner-music-ios\ios\LinerMusic\icons")

# (filename, pixel size)
SIZES = [
    ("AppIcon20x20@2x.png", 40),
    ("AppIcon20x20@3x.png", 60),
    ("AppIcon29x29@2x.png", 58),
    ("AppIcon29x29@3x.png", 87),
    ("AppIcon40x40@2x.png", 80),
    ("AppIcon40x40@3x.png", 120),
    ("AppIcon60x60@2x.png", 120),
    ("AppIcon60x60@3x.png", 180),
    ("AppIcon76x76@2x.png", 152),
    ("AppIcon83.5x83.5@2x.png", 167),
    ("AppIcon1024x1024.png", 1024),
]


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    source = Image.open(SRC).convert("RGBA")
    print(f"source: {SRC.name} {source.size}")

    for name, size in SIZES:
        icon = source.resize((size, size), Image.LANCZOS)
        target = OUT / name
        icon.save(target, format="PNG", optimize=True)
        print(f"  {name:28s} {size}x{size}  {target.stat().st_size} bytes")

    # The Settings/Launch background colour referenced by UILaunchScreen.
    print("done")


if __name__ == "__main__":
    main()
