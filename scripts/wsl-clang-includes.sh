#!/usr/bin/env bash
set -uo pipefail
D=/opt/swift/swift-6.4.2-RELEASE-ubuntu24.04/usr/lib/clang/21/include
echo "dir: $D"
ls "$D" 2>/dev/null | head -15
echo "count: $(ls "$D" 2>/dev/null | wc -l)"
echo "--- std headers ---"
ls "$D" 2>/dev/null | grep -i '^std' | head
echo "--- stdbool anywhere in toolchain ---"
find /opt/swift/swift-6.4.2-RELEASE-ubuntu24.04/usr/lib/clang -name 'stdbool.h' 2>/dev/null | head
