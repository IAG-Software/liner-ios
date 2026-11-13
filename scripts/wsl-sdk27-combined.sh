#!/usr/bin/env bash
set -uo pipefail
TC=/opt/swift/swift-6.4.2-RELEASE-ubuntu24.04
SDK=/opt/ios/sdk27/iPhoneOS27.2.sdk
S=$SDK/usr/lib/swift/shims
cd /tmp/swift-smoke

cat > t.swift <<'SWIFT'
import SwiftUI
struct V: View { var body: some View { Text("x") } }
SWIFT

rm -f t.o
"$TC/usr/bin/swiftc" -target arm64-apple-ios27.0 -sdk "$SDK" \
  -resource-dir "$SDK/usr/lib/swift" \
  -Xcc "-I$S" \
  -Xcc "-fmodule-map-file=$S/module.modulemap" \
  -parse-as-library -O -emit-object -o t.o t.swift 2>&1 | strings | head -12

if [ -f t.o ]; then echo "OBJECT OK: $(file -b t.o)"; else echo "NO OBJECT"; fi
