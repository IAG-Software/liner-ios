#!/usr/bin/env bash
# Third attempt: SDK Swift resource dir for modules, toolchain clang resource
# dir for builtin C headers, llvm-ar for the archive (no Apple libtool).
set -uo pipefail

TC=/opt/swift/swift-6.4.2-RELEASE-ubuntu24.04
SWIFTC=$TC/usr/bin/swiftc
AR=$TC/usr/bin/llvm-ar
SDKROOT=/opt/ios/sdk-src/iPhoneOS16.5.sdk
TARGET=arm64-apple-ios16.5
CLANG_RES=$TC/usr/lib/clang/21
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

rm -f smoke.o libsmoke.a

echo "=== emit object ==="
"$SWIFTC" -target "$TARGET" -sdk "$SDKROOT" \
  -resource-dir "$SDKROOT/usr/lib/swift" \
  -Xcc -isystem -Xcc "$CLANG_RES/include" \
  -parse-as-library -O -emit-object -o smoke.o smoke.swift 2>&1 | head -25

if [ -f smoke.o ]; then
  echo "object OK: $(file -b smoke.o)"
  echo
  echo "=== archive ==="
  "$AR" rcs libsmoke.a smoke.o
  echo "ar exit=$?"
  ls -l libsmoke.a
  echo
  echo "=== exported symbol ==="
  llvm-nm --defined-only libsmoke.a 2>/dev/null | grep -i liner_smoke
else
  echo "OBJECT FAILED"
fi
