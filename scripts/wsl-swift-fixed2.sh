#!/usr/bin/env bash
# Final approach: use the toolchain's default resource dir (clang finds its own
# builtin headers), and remove the toolchain's *Linux* Dispatch module map so it
# stops colliding with the one the iPhoneOS SDK ships.
set -uo pipefail

TC=/opt/swift/swift-6.4.2-RELEASE-ubuntu24.04
SWIFTC=$TC/usr/bin/swiftc
AR=$TC/usr/bin/llvm-ar
SDKROOT=/opt/ios/sdk-src/iPhoneOS16.5.sdk
TARGET=arm64-apple-ios16.5
WORK=/tmp/swift-smoke
mkdir -p "$WORK"
cd "$WORK"

DISPATCH_MAP=$TC/usr/lib/swift/dispatch/module.modulemap
if [ -f "$DISPATCH_MAP" ]; then
  mv "$DISPATCH_MAP" "$DISPATCH_MAP.linux-disabled"
  echo "disabled toolchain Dispatch module map"
fi

printf 'import SwiftUI\nstruct V: View { var body: some View { Text("hi").background(.ultraThinMaterial) } }\n@_cdecl("liner_smoke_ping") public func p() -> Int32 { 42 }\n' > smoke.swift

rm -f smoke.o libsmoke.a
echo "=== compile (default resource dir) ==="
"$SWIFTC" -target "$TARGET" -sdk "$SDKROOT" \
  -parse-as-library -O -emit-object -o smoke.o smoke.swift 2>&1 | grep -E "error:" | head -8

if [ -f smoke.o ]; then
  echo "OBJECT OK: $(file -b smoke.o)"
  "$AR" rcs libsmoke.a smoke.o
  echo "ARCHIVE OK: $(stat -c%s libsmoke.a) bytes"
  llvm-nm --defined-only libsmoke.a | grep -i liner_smoke | head -2
else
  echo "STILL FAILING"
fi
