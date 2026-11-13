#!/usr/bin/env bash
# The public theos stub SDK omits forward declarations that Apple's umbrella
# headers assume exist (NSError in NSArray.h is the first). Injecting them via a
# forced-include header lets Swift's clang importer build the Foundation module.
set -uo pipefail

TC=/opt/swift/swift-6.4.2-RELEASE-ubuntu24.04
SWIFTC=$TC/usr/bin/swiftc
SDKROOT=/opt/ios/sdk-src/iPhoneOS16.5.sdk
TARGET=arm64-apple-ios16.5
WORK=/tmp/swift-smoke
mkdir -p "$WORK"; cd "$WORK"

cat > shim.h <<'H'
#pragma once
#ifdef __OBJC__
@class NSError;
@class NSString;
@class NSURL;
@class NSData;
@class NSDictionary;
@class NSArray;
@class NSNumber;
@class NSCoder;
@class NSObject;
@class NSZone;
@class NSMethodSignature;
@class NSInvocation;
@class NSXPCConnection;
@class Protocol;
#endif
H

printf 'import SwiftUI\nstruct V: View { var body: some View { Text("hi").background(.ultraThinMaterial) } }\n@_cdecl("liner_ui") public func p() -> Int32 { 1 }\n' > ui.swift

rm -f ui.o
"$SWIFTC" -target "$TARGET" -sdk "$SDKROOT" \
  -Xcc -include -Xcc "$WORK/shim.h" \
  -parse-as-library -O -emit-object -o ui.o ui.swift 2>&1 | grep -E "error:" | head -6

if [ -f ui.o ]; then
  echo "SWIFTUI OK: $(file -b ui.o)"
else
  echo "--- still failing ---"
fi
