#!/usr/bin/env bash
# Capture the *full* diagnostic for the closest-to-working variant, instead of
# only grepping the first error line.
set -uo pipefail

TC=/opt/swift/swift-6.4.2-RELEASE-ubuntu24.04
SDK=/opt/ios/sdk27/iPhoneOS27.2.sdk
SHIMS="$SDK/usr/lib/swift/shims"
WORK=/tmp/swift-smoke
mkdir -p "$WORK"; cd "$WORK"

printf 'import SwiftUI\nstruct App: SwiftUI.App { var body: some Scene { WindowGroup { Text("x") } } }\n' > ui27.swift

echo "=== shims dir contents (modulemap + a header) ==="
ls "$SHIMS/module.modulemap" "$SHIMS/SwiftStdint.h" 2>&1 | head

echo
echo "=== full output: -resource-dir sdk ==="
"$TC/usr/bin/swiftc" -target arm64-apple-ios27.0 -sdk "$SDK" \
  -resource-dir "$SDK/usr/lib/swift" \
  -parse-as-library -O -emit-object -o /tmp/ui27.o ui27.swift 2>&1 | strings | head -25
