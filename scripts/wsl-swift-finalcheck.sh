#!/usr/bin/env bash
# Two checks:
#   1. does *pure* Swift (no Apple frameworks) compile for iOS?
#   2. can the Foundation umbrella be pre-included so header order resolves?
set -uo pipefail

TC=/opt/swift/swift-6.4.2-RELEASE-ubuntu24.04
SWIFTC=$TC/usr/bin/swiftc
SDKROOT=/opt/ios/sdk-src/iPhoneOS16.5.sdk
TARGET=arm64-apple-ios16.5
WORK=/tmp/swift-smoke
mkdir -p "$WORK"; cd "$WORK"

echo "=== 1. pure Swift ==="
printf 'public func add(_ a: Int, _ b: Int) -> Int { a + b }\n@_cdecl("liner_pure") public func p() -> Int32 { 7 }\n' > pure.swift
rm -f pure.o
"$SWIFTC" -target "$TARGET" -sdk "$SDKROOT" -parse-as-library -O -emit-object -o pure.o pure.swift 2>&1 | grep -E "error:" | head -4
if [ -f pure.o ]; then echo "PURE SWIFT OK: $(file -b pure.o)"; else echo "PURE SWIFT FAILED"; fi

echo
echo "=== 2. SwiftUI with Foundation pre-included ==="
printf 'import SwiftUI\nstruct V: View { var body: some View { Text("hi") } }\n' > ui.swift
rm -f ui.o
"$SWIFTC" -target "$TARGET" -sdk "$SDKROOT" \
  -Xcc -include -Xcc "$SDKROOT/System/Library/Frameworks/Foundation.framework/Headers/Foundation.h" \
  -parse-as-library -O -emit-object -o ui.o ui.swift 2>&1 | grep -E "error:" | head -4
if [ -f ui.o ]; then echo "SWIFTUI OK: $(file -b ui.o)"; else echo "SWIFTUI FAILED"; fi
