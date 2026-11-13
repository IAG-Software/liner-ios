#!/usr/bin/env bash
# Fetch a stripped iPhoneOS 27.2 SDK (headers + .tbd stubs + Swift interfaces)
# and test whether SwiftUI actually compiles against it.
set -uo pipefail

URL="https://github.com/xybp888/iOS-SDKs/releases/download/iOS27.2-SDKs/iPhoneOS27.2.sdk.zip"
DEST=/opt/ios/sdk27
mkdir -p "$DEST"

if [ ! -d "$DEST/iPhoneOS27.2.sdk" ]; then
  echo "downloading $URL"
  curl -fL --retry 3 -o /tmp/iPhoneOS27.2.sdk.zip "$URL" || exit 1
  echo "size: $(du -h /tmp/iPhoneOS27.2.sdk.zip | cut -f1)"
  unzip -q /tmp/iPhoneOS27.2.sdk.zip -d "$DEST" || exit 1
fi

SDK="$DEST/iPhoneOS27.2.sdk"
echo "sdk: $SDK"

echo "=== swift modules present? ==="
ls "$SDK/usr/lib/swift" 2>/dev/null | grep -E "^(Swift|SwiftUI|_Concurrency)\.swiftmodule$" || echo "none"
echo "=== SwiftUI swiftinterface? ==="
ls "$SDK/System/Library/Frameworks/SwiftUI.framework/Modules/SwiftUI.swiftmodule/" 2>/dev/null | head -4
echo "=== UIKit module map ==="
ls "$SDK/System/Library/Frameworks/UIKit.framework/Modules/" 2>/dev/null | head -3

TC=/opt/swift/swift-6.4.2-RELEASE-ubuntu24.04
WORK=/tmp/swift-smoke
mkdir -p "$WORK"; cd "$WORK"
printf 'import SwiftUI\nstruct V: View { var body: some View { Text("hi").background(.ultraThinMaterial) } }\n@_cdecl("liner_swiftui_probe") public func probe() -> Int32 { 1 }\n' > ui27.swift
rm -f ui27.o
echo "=== compile with 27.2 SDK ==="
"$TC/usr/bin/swiftc" -target arm64-apple-ios27.0 -sdk "$SDK" \
  -parse-as-library -O -emit-object -o ui27.o ui27.swift 2>&1 | grep -E "error:" | head -8
if [ -f ui27.o ]; then echo "SWIFTUI OK: $(file -b ui27.o)"; else echo "STILL FAILING"; fi
