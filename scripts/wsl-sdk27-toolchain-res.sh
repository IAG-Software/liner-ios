#!/usr/bin/env bash
# Two changes on a hunch:
#   * use the TOOLCHAIN's resource dir (it has a working SwiftShims) instead of
#     the trimmed SDK's incomplete stdlib wiring
#   * empty private-header stub (the earlier one imported UIKitDefines and made
#     a cycle, which killed UIKIT_EXTERN)
set -uo pipefail

TC=/opt/swift/swift-6.4.2-RELEASE-ubuntu24.04
SDK=/opt/ios/sdk27/iPhoneOS27.2.sdk
CLANG_RES=$TC/usr/lib/clang/21
PRIV=$SDK/System/Library/PrivateFrameworks
WORK=/tmp/swift-smoke
mkdir -p "$WORK"; cd "$WORK"

mkdir -p "$PRIV/UIUtilities.framework/Headers"
: > "$PRIV/UIUtilities.framework/Headers/UIDefines.h"

cat > t.swift <<'SWIFT'
import SwiftUI
struct V: View { var body: some View { Text("x").background(.ultraThinMaterial) } }
@_cdecl("liner_probe") public func probe() -> Int32 { 1 }
SWIFT

rm -f t.o
"$TC/usr/bin/swiftc" -target arm64-apple-ios27.0 -sdk "$SDK" \
  -resource-dir "$TC/usr/lib/swift" \
  -F "$SDK/System/Library/Frameworks" \
  -F "$PRIV" \
  -Xcc -nostdinc \
  -Xcc -isystem -Xcc "$CLANG_RES/include" \
  -Xcc -isystem -Xcc "$SDK/usr/include" \
  -Xcc -F"$PRIV" \
  -parse-as-library -O -emit-object -o t.o t.swift 2>&1 | grep -a "error:" | head -6

if [ -f t.o ]; then
  echo "SWIFTUI OK: $(file -b t.o)"
  llvm-nm --defined-only t.o 2>/dev/null | grep liner_probe | head -2
else
  echo "NO OBJECT"
fi
