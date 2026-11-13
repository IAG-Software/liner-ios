#!/usr/bin/env bash
# Solve both cross-compile problems at once:
#   * `-resource-dir` = the SDK's Swift dir  -> no duplicate Dispatch module map
#   * a `lib/clang` symlink inside that dir  -> clang still finds stdbool.h & co
set -uo pipefail

TC=/opt/swift/swift-6.4.2-RELEASE-ubuntu24.04
SWIFTC=$TC/usr/bin/swiftc
AR=$TC/usr/bin/llvm-ar
SDKROOT=/opt/ios/sdk-src/iPhoneOS16.5.sdk
TARGET=arm64-apple-ios16.5
WORK=/tmp/swift-smoke
mkdir -p "$WORK"
cd "$WORK"

# clang resolves builtins at <resource-dir>/lib/clang/<ver>/include
mkdir -p "$SDKROOT/usr/lib/swift/lib"
ln -sfn "$TC/usr/lib/clang" "$SDKROOT/usr/lib/swift/lib/clang"
echo "symlink: $(readlink -f "$SDKROOT/usr/lib/swift/lib/clang")"
echo "builtin: $(ls "$SDKROOT/usr/lib/swift/lib/clang/21/include/stdbool.h" 2>/dev/null || echo MISSING)"

printf 'import SwiftUI\nstruct V: View { var body: some View { Text("hi").background(.ultraThinMaterial) } }\n@_cdecl("liner_smoke_ping") public func p() -> Int32 { 42 }\n' > smoke.swift

rm -f smoke.o libsmoke.a
echo
echo "=== compile ==="
"$SWIFTC" -target "$TARGET" -sdk "$SDKROOT" \
  -resource-dir "$SDKROOT/usr/lib/swift" \
  -parse-as-library -O -emit-object -o smoke.o smoke.swift 2>&1 | grep -E "error:" | head -6

if [ -f smoke.o ]; then
  echo "OBJECT OK: $(file -b smoke.o)"
  "$AR" rcs libsmoke.a smoke.o
  echo "ARCHIVE OK: $(ls -l libsmoke.a | awk '{print $5}') bytes"
  llvm-nm --defined-only libsmoke.a | grep -i liner_smoke | head -2
else
  echo "STILL FAILING"
fi
