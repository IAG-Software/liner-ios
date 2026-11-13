#!/usr/bin/env bash
# Complete the stripped SDK:
#  1. it ships without `shims/` (SwiftShims lives there) -> copy from toolchain
#  2. its interfaces record Apple's compiler build -> rewrite to ours so the
#     module loader's version gate passes
set -uo pipefail

TC=/opt/swift/swift-6.4.2-RELEASE-ubuntu24.04
SDK=/opt/ios/sdk27/iPhoneOS27.2.sdk
WORK=/tmp/swift-smoke
mkdir -p "$WORK"; cd "$WORK"

if [ ! -d "$SDK/usr/lib/swift/shims" ]; then
  cp -r "$TC/usr/lib/swift/shims" "$SDK/usr/lib/swift/shims"
  echo "copied shims into SDK"
fi

OURS="Swift version 6.4.2 (swift-6.4.2-RELEASE)"

patched=0
while IFS= read -r -d '' f; do
  if grep -q "swift-compiler-version:" "$f"; then
    sed -i "s|^// swift-compiler-version:.*|// swift-compiler-version: ${OURS}|" "$f"
    sed -i "s|-user-module-version [0-9.]*|-user-module-version 6.4.2|" "$f"
    patched=$((patched + 1))
  fi
done < <(find "$SDK/usr/lib/swift" -name "*.swiftinterface" -print0)
echo "patched $patched interfaces"

printf 'import SwiftUI\nstruct V: View { var body: some View { Text("hi").background(.ultraThinMaterial) } }\n@_cdecl("liner_swiftui_probe") public func probe() -> Int32 { 1 }\n' > ui27.swift

rm -f ui27.o
echo "=== compile ==="
"$TC/usr/bin/swiftc" -target arm64-apple-ios27.0 -sdk "$SDK" \
  -parse-as-library -O -emit-object -o ui27.o ui27.swift 2>&1 | grep -E "error:" | head -6
if [ -f ui27.o ]; then echo "SWIFTUI OK: $(file -b ui27.o)"; else echo "STILL FAILING"; fi
