#!/usr/bin/env bash
# The SDK's `_c_standard_library_obsolete` module refuses to build when the host
# (glibc) headers are visible: Apple's module maps gate on a feature that the
# compiler sets when it detects incompatible search paths. Isolate the include
# path to the SDK + clang's own builtins, exactly like a real iOS toolchain.
set -uo pipefail

TC=/opt/swift/swift-6.4.2-RELEASE-ubuntu24.04
SDK=/opt/ios/sdk27/iPhoneOS27.2.sdk
CLANG_RES=$TC/usr/lib/clang/21
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
  -Xcc -nostdinc \
  -Xcc -isystem -Xcc "$SDK/usr/include" \
  -Xcc -isystem -Xcc "$CLANG_RES/include" \
  -Xcc -I"$SDK/usr/lib/swift/shims" \
  -parse-as-library -O -emit-object -o t.o t.swift 2>&1 | grep -a "error:" | head -8

if [ -f t.o ]; then
  echo "SWIFTUI OK: $(file -b t.o)"
  /usr/bin/llvm-nm --defined-only t.o 2>/dev/null | grep liner_probe | head -2
else
  echo "NO OBJECT"
fi
