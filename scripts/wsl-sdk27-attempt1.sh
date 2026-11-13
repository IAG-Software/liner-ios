#!/usr/bin/env bash
# The 27.2 SDK's Swift interfaces were built by Apple's Xcode toolchain, so the
# compiler refuses them on a version check, and SwiftShims lives in the toolchain
# rather than the SDK. Try to satisfy both.
set -uo pipefail

TC=/opt/swift/swift-6.4.2-RELEASE-ubuntu24.04
SDK=/opt/ios/sdk27/iPhoneOS27.2.sdk
WORK=/tmp/swift-smoke
mkdir -p "$WORK"; cd "$WORK"

echo "=== toolchain version string ==="
"$TC/usr/bin/swiftc" -version 2>&1 | head -2

echo
echo "=== current interface header (Swift module) ==="
head -3 "$SDK/usr/lib/swift/Swift.swiftmodule/arm64e-apple-ios.swiftinterface"

echo
echo "=== shims available in toolchain? ==="
ls "$TC/usr/lib/swift/shims/module.modulemap" 2>/dev/null || echo "no shims modulemap"

printf 'import SwiftUI\nstruct V: View { var body: some View { Text("hi").background(.ultraThinMaterial) } }\n@_cdecl("liner_swiftui_probe") public func probe() -> Int32 { 1 }\n' > ui27.swift

echo
echo "=== attempt: shims path only ==="
rm -f ui27.o
"$TC/usr/bin/swiftc" -target arm64-apple-ios27.0 -sdk "$SDK" \
  -Xcc -I"$TC/usr/lib/swift/shims" \
  -parse-as-library -O -emit-object -o ui27.o ui27.swift 2>&1 | grep -E "error:" | head -6
[ -f ui27.o ] && echo "OK with shims path" || echo "still failing"
