#!/usr/bin/env bash
# The previous attempt proved two things:
#   * with shims on the include path, the SwiftShims module BUILDS (we saw its
#     warnings) — so that part works
#   * the remaining failure was header-search pollution: Linux's headers made
#     Apple's `_c_standard_library_obsolete` module refuse to build
# So: isolate the C search path to the SDK, restore framework search (-F), and
# keep the shims path that was already working.
set -uo pipefail

TC=/opt/swift/swift-6.4.2-RELEASE-ubuntu24.04
SDK=/opt/ios/sdk27/iPhoneOS27.2.sdk
CLANG_RES=$TC/usr/lib/clang/21
SHIMS=$SDK/usr/lib/swift/shims
WORK=/tmp/swift-smoke
mkdir -p "$WORK"; cd "$WORK"

cat > t.swift <<'SWIFT'
import SwiftUI
struct V: View { var body: some View { Text("x").background(.ultraThinMaterial) } }
@_cdecl("liner_probe") public func probe() -> Int32 { 1 }
SWIFT

try() {
  local name="$1"; shift
  rm -f t.o
  if "$TC/usr/bin/swiftc" -target arm64-apple-ios27.0 -sdk "$SDK" "$@" \
       -parse-as-library -O -emit-object -o t.o t.swift >/tmp/sw.log 2>&1; then
    echo "PASS: $name"
    return 0
  fi
  echo "FAIL: $name"
  grep -a "error:" /tmp/sw.log | head -3 | sed 's/^/    /'
  return 1
}

COMMON=(
  -resource-dir "$SDK/usr/lib/swift"
  -F "$SDK/System/Library/Frameworks"
  -Xcc -nostdinc
  -Xcc -isysroot -Xcc "$SDK"
  -Xcc -I"$SDK/usr/include"
  -Xcc -I"$CLANG_RES/include"
  -Xcc -I"$SHIMS"
)

try "isolated search + frameworks" "${COMMON[@]}"
try "isolated + explicit module map" "${COMMON[@]}" -Xcc -fmodule-map-file="$SHIMS/module.modulemap"

if [ -f t.o ]; then
  echo
  echo "RESULT: $(file -b t.o)"
  llvm-nm --defined-only t.o 2>/dev/null | grep liner_probe | head -2
fi
