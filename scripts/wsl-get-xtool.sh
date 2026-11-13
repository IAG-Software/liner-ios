#!/usr/bin/env bash
# xtool ships a patched Swift driver, which is the piece the stock Linux
# toolchain lacks. Pull the AppImage, unpack it (no FUSE needed), and see what
# is actually inside.
set -uo pipefail

URL="https://github.com/xtool-org/xtool/releases/latest/download/xtool-x86_64.AppImage"
DEST=/opt/xtool
mkdir -p "$DEST"

if [ ! -x "$DEST/xtool.AppImage" ]; then
  echo "downloading xtool AppImage"
  curl -fL --retry 3 -o "$DEST/xtool.AppImage" "$URL" || exit 1
  chmod +x "$DEST/xtool.AppImage"
fi
echo "size: $(du -h "$DEST/xtool.AppImage" | cut -f1)"

if [ ! -d "$DEST/squashfs-root" ]; then
  cd "$DEST"
  ./xtool.AppImage --appimage-extract >/dev/null 2>&1 || true
  mv squashfs-root extracted 2>/dev/null || true
fi

echo "=== top level ==="
ls "$DEST/extracted" 2>/dev/null | head -15

echo
echo "=== binaries of interest ==="
find "$DEST/extracted" -maxdepth 4 -type f \( -name "swift*" -o -name "xtool" -o -name "ldid" \) 2>/dev/null | head -20

echo
echo "=== usr/bin ==="
ls "$DEST/extracted/usr/bin" 2>/dev/null | head -30
