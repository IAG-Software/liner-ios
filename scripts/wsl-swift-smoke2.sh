#!/usr/bin/env bash
# Retry with the SDK's Swift resource dir (removes the toolchain's Dispatch
# module map from the search path) and llvm-ar instead of Apple's libtool.
set -uo pipefail

SWIFTC=/opt/swift/swift-6.4.2-RELEASE-ubuntu24.04/usr/bin/swiftc
AR=/opt/swift/swift-6.4.2-RELEASE-ubuntu24.04/usr/bin/llvm-ar
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

echo "=== emit object (resource-dir = SDK) ==="
"$SWIFTC" -target "$TARGET" -sdk "$SDKROOT" \
  -resource-dir "$SDKROOT/usr/lib/swift" \
  -parse-as-library -O -emit-object -o smoke.o smoke.swift 2>&1 | head -25
ls -l smoke.o 2>/dev/null && file smoke.o

echo
echo "=== archive with llvm-ar ==="
rm -f libsmoke.a
"$AR" rcs libsmoke.a smoke.o && ls -l libsmoke.a

echo
echo "=== symbol ==="
"$AR" t libsmoke.a | head -3
llvm-nm --defined-only libsmoke.a 2>/dev/null | grep -i liner_smoke
