#!/usr/bin/env bash
set -uo pipefail
S=/opt/ios/sdk-src/iPhoneOS16.5.sdk

echo "=== swiftmodule count in usr/lib/swift ==="
ls "$S/usr/lib/swift" | grep -c "swiftmodule" || true

echo
echo "=== stdlib / SwiftUI modules ==="
ls "$S/usr/lib/swift" | grep -E "^(Swift|SwiftUI|_Concurrency|Observation)\." || echo "none at top level"

echo
echo "=== SwiftUI framework contents ==="
ls "$S/System/Library/Frameworks/SwiftUI.framework" 2>/dev/null || echo "no SwiftUI.framework"

echo
echo "=== swift runtime tbd stubs ==="
ls "$S/usr/lib/swift" | grep -E "^libswift(Core|Foundation|UI|_Concurrency)" | head -10

echo
echo "=== UIKit swiftinterface? ==="
ls "$S/System/Library/Frameworks/UIKit.framework/Modules" 2>/dev/null | head
