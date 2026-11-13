#!/usr/bin/env bash
# Prove the frontend is embedded (custom-protocol) rather than loaded from devUrl.
set -uo pipefail
BIN=/opt/ios/src/build-ios/Payload/LinerMusic.app/LinerMusic
IPA=/opt/ios/src/build-ios/LinerMusic-unsigned-ios.ipa

echo "binary: $(du -h "$BIN" | cut -f1)   ipa: $(du -h "$IPA" | cut -f1)"

echo
echo "=== EmbeddedAssets monomorphisation present? ==="
llvm-nm "$BIN" 2>/dev/null | grep -c -i "embeddedassets" || true

echo
echo "=== asset-catalogue symbols (tauri-codegen phf tables) ==="
llvm-nm "$BIN" 2>/dev/null | grep -iE "ASSETS|asset_resolver|__TAURI" | head -8

echo
echo "=== index.html occurrences (static keys + embedded map) ==="
strings -a "$BIN" | grep -c "index.html" || true

echo
echo "=== brotli-compressed blob heuristic: assets section entropy size ==="
llvm-objdump --macho --private-headers "$BIN" 2>/dev/null | grep -A3 "sectname __const" | head -8
