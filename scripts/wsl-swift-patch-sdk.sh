#!/usr/bin/env bash
# Patch the local stub SDK's NSArray.h to forward-declare NSError (Apple's SDK
# gets this from a header the theos stub set omits), then retry SwiftUI.
set -uo pipefail

TC=/opt/swift/swift-6.4.2-RELEASE-ubuntu24.04
SWIFTC=$TC/usr/bin/swiftc
SDKROOT=/opt/ios/sdk-src/iPhoneOS16.5.sdk
F=$SDKROOT/System/Library/Frameworks/Foundation.framework/Headers
TARGET=arm64-apple-ios16.5
WORK=/tmp/swift-smoke
mkdir -p "$WORK"; cd "$WORK"

# idempotent patch
if ! grep -q '^@class NSError;' "$F/NSArray.h"; then
  sed -i 's/^@class NSData, NSIndexSet, NSString, NSURL;/@class NSData, NSIndexSet, NSString, NSURL;\n@class NSError;/' "$F/NSArray.h"
  echo "patched NSArray.h"
fi
grep -n "^@class" "$F/NSArray.h" | head -3

printf 'import SwiftUI\nstruct V: View { var body: some View { Text("hi").background(.ultraThinMaterial) } }\n@_cdecl("liner_ui") public func p() -> Int32 { 1 }\n' > ui.swift

rm -f ui.o
"$SWIFTC" -target "$TARGET" -sdk "$SDKROOT" -parse-as-library -O -emit-object -o ui.o ui.swift 2>&1 \
  | grep -E "error:" | head -8

if [ -f ui.o ]; then echo "SWIFTUI OK: $(file -b ui.o)"; else echo "--- failing ---"; fi
