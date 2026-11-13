#!/usr/bin/env bash
set -uo pipefail
R=/opt/rust/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f

echo "=== tauri-macros: mobile_entry_point (ios) ==="
grep -rn "start_app" "$R/tauri-macros-2.7.4/src/" | head -10

echo
echo "=== tauri lib.rs: start_app / run_mobile ==="
grep -rn "start_app\|UIApplicationMain\|fn run_mobile" "$R/tauri-2.12.5/src/lib.rs" | head -20

echo
echo "=== tauri ios.rs full ==="
sed -n '1,80p' "$R/tauri-2.12.5/src/ios.rs"

echo
echo "=== tauri-runtime-wry ios ==="
ls "$R/tauri-runtime-wry-2.12.3/src/"
grep -rn "UIApplicationMain\|start_app\|UIWindow\|UIWindowScene\|app_delegate" "$R/tauri-runtime-wry-2.12.3/src/ios.rs" 2>/dev/null | head -30
