#!/usr/bin/env bash
set -uo pipefail
BIN=/opt/ios/src/build-ios/Payload/LinerMusic.app/LinerMusic
echo "=== native overlay class present? ==="
llvm-nm --defined-only "$BIN" 2>/dev/null | grep -c "LinerLyricsOverlayController" || true
echo "=== C symbols ==="
llvm-nm --defined-only "$BIN" 2>/dev/null | grep "liner_lyrics" | head -4
echo "=== UIKit material string ==="
strings -a "$BIN" | grep -c "LinerLyricsOverlay" || true
