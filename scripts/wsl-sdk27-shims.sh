#!/usr/bin/env bash
# SwiftShims already exists in the SDK now; the importer just needs the include
# path so its module map is discovered.
set -uo pipefail

TC=/opt/swift/swift-6.4.2-RELEASE-ubuntu24.04
SDK=/opt/ios/sdk27/iPhoneOS27.2.sdk
SHIMS="$SDK/usr/lib/swift/shims"
WORK=/tmp/swift-smoke
mkdir -p "$WORK"; cd "$WORK"

printf 'import SwiftUI\nstruct V: View { var body: some View { Text("hi").background(.ultraThinMaterial) } }\n@_cdecl("liner_swiftui_probe") public func probe() -> Int32 { 1 }\n' > ui27.swift

rm -f ui27.o
echo "=== compile with shims include path ==="
"$TC/usr/bin/swiftc" -target arm64-apple-ios27.0 -sdk "$SDK" \
  -Xcc -I"$SHIMS" \
  -parse-as-library -O -emit-object -o ui27.o ui27.swift 2>&1 | grep -E "error:" | head -8
if [ -f ui27.o ]; then echo "SWIFTUI OK: $(file -b ui27.o)"; else echo "STILL FAILING"; fi
