#!/usr/bin/env bash
set -uo pipefail
R=/opt/rust/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f
TAO=$(ls -d "$R"tao-0.37.2 2>/dev/null | head -1)
echo "tao: $TAO"
echo "=== ios backend files ==="
ls "$TAO/src/platform_impl/ios/" 2>/dev/null

echo
echo "=== UIApplicationMain in tao ==="
grep -rn "UIApplicationMain\|AppDelegate\|UIWindowScene\|UIApplicationDelegate" "$TAO/src/platform_impl/ios/" 2>/dev/null | head -25

echo
echo "=== tao EventLoop::run (ios) ==="
sed -n '1,60p' "$TAO/src/platform_impl/ios/event_loop.rs" 2>/dev/null | head -60
