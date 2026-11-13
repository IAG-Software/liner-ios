#!/usr/bin/env bash
# Find a working swiftc invocation for arm64-apple-ios SwiftUI.
# Variants differ in how the clang builtin headers (stdbool.h) are found and
# how the duplicate Dispatch module map is avoided.
set -uo pipefail

TC=/opt/swift/swift-6.4.2-RELEASE-ubuntu24.04
SWIFTC=$TC/usr/bin/swiftc
AR=$TC/usr/bin/llvm-ar
SDKROOT=/opt/ios/sdk-src/iPhoneOS16.5.sdk
CLANG_RES=$TC/usr/lib/clang/21
TARGET=arm64-apple-ios16.5
WORK=/tmp/swift-smoke
mkdir -p "$WORK"
cd "$WORK"

printf 'import SwiftUI\nstruct V: View { var body: some View { Text("hi") } }\n@_cdecl("liner_smoke_ping") public func p() -> Int32 { 42 }\n' > smoke.swift

try() {
  local name=$1; shift
  rm -f smoke.o
  if "$SWIFTC" "$@" -parse-as-library -O -emit-object -o smoke.o smoke.swift >/tmp/swift-try.log 2>&1; then
    echo "PASS: $name"
    return 0
  else
    echo "FAIL: $name"
    grep -m2 -E "error:" /tmp/swift-try.log | sed 's/^/    /'
    return 1
  fi
}

BASE=(-target "$TARGET" -sdk "$SDKROOT")

# 1. toolchain resource dir (default) + explicit clang include
try "default-resource + -Xcc -isystem" \
  "${BASE[@]}" -Xcc -isystem -Xcc "$CLANG_RES/include"

# 2. explicit clang resource dir, equals form
try "default-resource + -Xcc -resource-dir=" \
  "${BASE[@]}" -Xcc "-resource-dir=$CLANG_RES"

# 3. Xclang resource dir
try "default-resource + -Xclang -resource-dir" \
  "${BASE[@]}" -Xclang -resource-dir -Xclang "$CLANG_RES"

# 4. SDK resource dir + clang include both
try "sdk-resource + -Xcc -isystem" \
  "${BASE[@]}" -resource-dir "$SDKROOT/usr/lib/swift" \
  -Xcc -isystem -Xcc "$CLANG_RES/include"

echo
if [ -f smoke.o ]; then
  echo "object: $(file -b smoke.o)"
  "$AR" rcs libsmoke.a smoke.o && llvm-nm --defined-only libsmoke.a | grep -i liner_smoke | head -2
fi
