#!/usr/bin/env bash
# Smoke test: can the Linux swiftc emit arm64-apple-ios objects that import
# SwiftUI, against the iPhoneOS SDK we already use for the ObjC host?
set -uo pipefail

SWIFTC=/opt/swift/swift-6.4.2-RELEASE-ubuntu24.04/usr/bin/swiftc
SDKROOT=/opt/ios/sdk-src/iPhoneOS16.5.sdk
TARGET=arm64-apple-ios16.5
WORK=/tmp/swift-smoke
mkdir -p "$WORK"
cd "$WORK"

cat > smoke.swift <<'SWIFT'
import SwiftUI

struct SmokeView: View {
    var body: some View {
        Text("hi").padding().background(.ultraThinMaterial)
    }
}

@_cdecl("liner_smoke_ping")
public func liner_smoke_ping() -> Int32 { 42 }
SWIFT

echo "=== emit object ==="
"$SWIFTC" -target "$TARGET" -sdk "$SDKROOT" -parse-as-library -O \
  -emit-object -o smoke.o smoke.swift 2>&1 | head -20
echo "object exit=$?"
ls -l smoke.o 2>/dev/null && file smoke.o

echo
echo "=== emit static archive ==="
"$SWIFTC" -target "$TARGET" -sdk "$SDKROOT" -parse-as-library -O \
  -module-name LinerSmoke -emit-library -static -o libsmoke.a smoke.swift 2>&1 | head -20
echo "archive exit=$?"
ls -l libsmoke.a 2>/dev/null && file libsmoke.a

echo
echo "=== exported C symbol ==="
llvm-nm libsmoke.a 2>/dev/null | grep -i liner_smoke | head -3
