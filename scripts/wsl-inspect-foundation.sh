#!/usr/bin/env bash
set -uo pipefail
SDK=/opt/ios/sdk-src/iPhoneOS16.5.sdk
F=$SDK/System/Library/Frameworks/Foundation.framework/Headers

echo "=== NSArray.h first 30 lines ==="
sed -n '1,30p' "$F/NSArray.h"

echo
echo "=== does NSArray.h forward-declare NSError? ==="
grep -n "NSError" "$F/NSArray.h" | head -5

echo
echo "=== Foundation.h include order (first 40 includes) ==="
grep -n "#import\|#include" "$F/Foundation.h" | head -40

echo
echo "=== module map? ==="
ls "$SDK/System/Library/Frameworks/Foundation.framework/Modules/" 2>/dev/null
