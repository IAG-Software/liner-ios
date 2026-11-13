#!/usr/bin/env bash
set -uo pipefail
BIN=/opt/ios/src/build-ios/Payload/LinerMusic.app/LinerMusic
echo "stub marker present: $(strings -a "$BIN" | grep -c 'native lyrics overlay unavailable')"
echo "swift archive:"
ls /opt/ios/src/src-tauri/target/aarch64-apple-ios/release/build/liner-music/*/out/libLinerLyrics.a 2>/dev/null || echo "  none (SDK cannot import SwiftUI)"
echo "swift overlay symbols in binary:"
llvm-nm --defined-only "$BIN" 2>/dev/null | grep -c "liner_lyrics" || true
