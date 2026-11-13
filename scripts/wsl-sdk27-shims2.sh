#!/usr/bin/env bash
set -uo pipefail

TC=/opt/swift/swift-6.4.2-RELEASE-ubuntu24.04
SDK=/opt/ios/sdk27/iPhoneOS27.2.sdk
CLANG_RES=$TC/usr/lib/clang/21
SHIMS=$TC/usr/lib/swift/shims
PRIV=$SDK/System/Library/PrivateFrameworks
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
  if "$TC/usr/bin/swiftc" -target arm64-apple-ios27.0 -sdk "$SDK" \
      -resource-dir "$TC/usr/lib/swift" \
      -F "$SDK/System/Library/Frameworks" -F "$PRIV" \
      -Xcc -nostdinc \
      -Xcc -isystem -Xcc "$CLANG_RES/include" \
      -Xcc -isystem -Xcc "$SDK/usr/include" \
      -Xcc -F"$PRIV" \
      "$@" \
      -parse-as-library -O -emit-object -o t.o t.swift >/tmp/sw.log 2>&1; then
    echo "PASS: $name"
    return 0
  fi
  echo "FAIL: $name"
  grep -a "error:" /tmp/sw.log | head -3 | sed 's/^/    /'
  return 1
}

try "shims include" -Xcc -I"$SHIMS"
try "shims modulemap" -Xcc -fmodule-map-file="$SHIMS/module.modulemap"
try "both shims flags" -Xcc -I"$SHIMS" -Xcc -fmodule-map-file="$SHIMS/module.modulemap"

if [ -f t.o ]; then
  echo; echo "RESULT: $(file -b t.o)"
  llvm-nm --defined-only t.o 2>/dev/null | grep liner_probe | head -2
fi
