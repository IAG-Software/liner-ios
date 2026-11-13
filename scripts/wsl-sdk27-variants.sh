#!/usr/bin/env bash
# Try the remaining ways to make `import SwiftShims` resolve inside the SDK's
# stdlib interface.
set -uo pipefail

TC=/opt/swift/swift-6.4.2-RELEASE-ubuntu24.04
SDK=/opt/ios/sdk27/iPhoneOS27.2.sdk
SHIMS="$SDK/usr/lib/swift/shims"
WORK=/tmp/swift-smoke
mkdir -p "$WORK"; cd "$WORK"

printf 'import SwiftUI\nstruct V: View { var body: some View { Text("hi") } }\n' > ui27.swift

try() {
  local name="$1"; shift
  rm -f ui27.o
  if "$TC/usr/bin/swiftc" -target arm64-apple-ios27.0 -sdk "$SDK" "$@" \
       -parse-as-library -O -emit-object -o ui27.o ui27.swift >/tmp/sw.log 2>&1; then
    echo "PASS: $name"
    return 0
  fi
  echo "FAIL: $name"
  grep -m1 "error:" /tmp/sw.log | sed 's/^/    /'
  return 1
}

try "explicit module map" -Xcc -fmodule-map-file="$SHIMS/module.modulemap"
try "swift -I shims"      -I "$SHIMS"
try "both"                -I "$SHIMS" -Xcc -fmodule-map-file="$SHIMS/module.modulemap"
try "resource-dir = sdk"  -resource-dir "$SDK/usr/lib/swift"
try "resource-dir + clang include" \
    -resource-dir "$SDK/usr/lib/swift" \
    -Xcc -isystem -Xcc "$TC/usr/lib/clang/21/include"

if [ -f ui27.o ]; then echo "RESULT: $(file -b ui27.o)"; fi
