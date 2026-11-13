#!/usr/bin/env bash
# Apple ships stub float.h/stddef.h in the SDK and gates them behind a module
# that only exists when search paths are wrong. Ours resolved the stub instead of
# clang's builtin because the SDK include dir came first. Put clang's builtins
# ahead of it.
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

rm -f t.o
"$TC/usr/bin/swiftc" -target arm64-apple-ios27.0 -sdk "$SDK" \
  -resource-dir "$SDK/usr/lib/swift" \
  -F "$SDK/System/Library/Frameworks" \
  -Xcc -nostdinc \
  -Xcc -isystem -Xcc "$CLANG_RES/include" \
  -Xcc -isystem -Xcc "$SDK/usr/include" \
  -Xcc -I"$SHIMS" \
  -parse-as-library -O -emit-object -o t.o t.swift 2>&1 | grep -a "error:" | head -6

if [ -f t.o ]; then
  echo "SWIFTUI OK: $(file -b t.o)"
  llvm-nm --defined-only t.o 2>/dev/null | grep liner_probe | head -2
else
  echo "NO OBJECT"
fi
