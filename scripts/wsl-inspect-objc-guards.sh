#!/usr/bin/env bash
set -uo pipefail
SDK=/opt/ios/sdk-src/iPhoneOS16.5.sdk
F=$SDK/System/Library/Frameworks/Foundation.framework/Headers

echo "=== NSObjCRuntime.h: NSError forward decl? ==="
grep -n "NSError" "$F/NSObjCRuntime.h" | head -5

echo
echo "=== NSObjCRuntime.h: __OBJC__ guard? ==="
grep -n "__OBJC__" "$F/NSObjCRuntime.h" | head -5

echo
echo "=== NSObject.h: __OBJC__ guard ==="
grep -n "__OBJC__" "$F/NSObject.h" | head -5

echo
echo "=== Foundation module map ==="
cat "$SDK/System/Library/Frameworks/Foundation.framework/Modules/module.modulemap" 2>/dev/null | head -20
