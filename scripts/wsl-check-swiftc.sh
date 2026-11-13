#!/usr/bin/env bash
set -uo pipefail
BIN=/opt/swift/swift-6.4.2-RELEASE-ubuntu24.04/usr/bin
echo "=== swiftc present? ==="
ls -l "$BIN/swiftc" 2>&1 || echo "MISSING swiftc"

echo
echo "=== swift-driver --version ==="
"$BIN/swiftc" --version 2>&1 | head -5
echo "exit=$?"

echo
echo "=== shared lib deps ==="
ldd "$BIN/swift-frontend" 2>&1 | grep -c "not found" || true
ldd "$BIN/swift-frontend" 2>&1 | grep "not found" | head -8
