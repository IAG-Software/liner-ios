#!/usr/bin/env bash
# Confirm the diagnostic build actually contains the boot guard and the log command.
set -uo pipefail
BIN=/opt/ios/src/build-ios/Payload/LinerMusic.app/LinerMusic

echo "boot-guard markers:"
strings -a "$BIN" | grep -c "boot-guard" || true
strings -a "$BIN" | grep -m2 "booting" || true

echo
echo "boot_log command + file name:"
strings -a "$BIN" | grep -m4 -E "boot_log|liner-boot\.log|React did not mount" || true

echo
echo "webview/asset origin:"
strings -a "$BIN" | grep -m3 "tauri://localhost" || true
