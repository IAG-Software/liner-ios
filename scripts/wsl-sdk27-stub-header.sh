#!/usr/bin/env bash
# The stripped SDK omits a private header that public UIKit headers import.
# Provide a minimal stub framework and retry; this is the last missing header
# between us and a SwiftUI object.
set -uo pipefail

TC=/opt/swift/swift-6.4.2-RELEASE-ubuntu24.04
SDK=/opt/ios/sdk27/iPhoneOS27.2.sdk
CLANG_RES=$TC/usr/lib/clang/21
SHIMS=$SDK/usr/lib/swift/shims
PRIV=$SDK/System/Library/PrivateFrameworks
WORK=/tmp/swift-smoke
mkdir -p "$WORK"; cd "$WORK"

# stub the private framework header UIKit imports
mkdir -p "$PRIV/UIUtilities.framework/Headers"
cat > "$PRIV/UIUtilities.framework/Headers/UIDefines.h" <<'EOF'
#pragma once
#import <UIKit/UIKitDefines.h>
EOF
echo "stubbed UIDefines.h"

cat > t.swift <<'SWIFT'
import SwiftUI
struct V: View { var body: some View { Text("x").background(.ultraThinMaterial) } }
@_cdecl("liner_probe") public func probe() -> Int32 { 1 }
SWIFT

rm -f t.o
"$TC/usr/bin/swiftc" -target arm64-apple-ios27.0 -sdk "$SDK" \
  -resource-dir "$SDK/usr/lib/swift" \
  -F "$SDK/System/Library/Frameworks" \
  -F "$PRIV" \
  -Xcc -nostdinc \
  -Xcc -isystem -Xcc "$CLANG_RES/include" \
  -Xcc -isystem -Xcc "$SDK/usr/include" \
  -Xcc -I"$SHIMS" \
  -Xcc -F"$PRIV" \
  -parse-as-library -O -emit-object -o t.o t.swift 2>&1 | grep -a "error:" | head -6

if [ -f t.o ]; then
  echo "SWIFTUI OK: $(file -b t.o)"
  llvm-nm --defined-only t.o 2>/dev/null | grep liner_probe | head -2
else
  echo "NO OBJECT"
fi
