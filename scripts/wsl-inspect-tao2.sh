#!/usr/bin/env bash
set -uo pipefail
R=/opt/rust/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f
echo "=== tao dirs ==="
ls -d "$R"tao* 2>/dev/null
TAO=$(ls -d "$R"tao-* 2>/dev/null | head -1)
echo "tao: $TAO"
echo
echo "=== ios platform files ==="
find "$TAO/src" -maxdepth 3 -type d -name "ios*" 2>/dev/null
find "$TAO/src" -maxdepth 4 -name "*ios*" 2>/dev/null | head -20
echo
echo "=== UIApplicationMain anywhere in tao ==="
grep -rn "UIApplicationMain" "$TAO/src" 2>/dev/null | head -10
echo
echo "=== tao deps that might hold ios backend (wry) ==="
WRY=$(ls -d "$R"wry-* 2>/dev/null | head -1)
echo "wry: $WRY"
grep -rn "UIApplicationMain\|UIWindowScene\|app_delegate" "$WRY/src" 2>/dev/null | head -15
