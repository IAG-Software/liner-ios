#!/usr/bin/env bash
set -euo pipefail
R=/opt/rust/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f
echo "=== who defines log_stdout / on_webview_created / run_plugin_command ==="
grep -rn "log_stdout\|on_webview_created\|run_plugin_command" "$R/tauri-2.12.5/src/" | head -20
echo
echo "=== swift files shipped in tauri crate ==="
find "$R/tauri-2.12.5" -name "*.swift" | head -20
echo
echo "=== tauri crate build.rs ios handling ==="
grep -n "swift\|ios" "$R/tauri-2.12.5/build.rs" | head -30
echo
echo "=== swift-rs symbols ==="
grep -rn "release_object\|retain_object\|string_from_bytes" "$R/swift-rs-1.0.9/src/" | head -10
