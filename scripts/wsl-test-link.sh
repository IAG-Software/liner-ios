#!/usr/bin/env bash
# Verify that clang can link a Mach-O arm64 binary with lld on this host.
set -euo pipefail

SDKROOT=/opt/ios/sdk-src/iPhoneOS16.5.sdk
export SDKROOT
mkdir -p /opt/ios/bin /tmp/linktest
cd /tmp/linktest

echo "--- lld flavours available ---"
ls -l /usr/bin/*lld* 2>/dev/null || true

cat > hello.c <<'C'
int main(void) { return 0; }
C

echo "--- compile ---"
clang -target arm64-apple-ios16.5 -isysroot "$SDKROOT" -arch arm64 -c hello.c -o hello.o
file hello.o

echo "--- link with -fuse-ld=lld ---"
set +e
clang -target arm64-apple-ios16.5 -isysroot "$SDKROOT" -arch arm64 \
  -fuse-ld=lld -nostartfiles -Wl,-e,_main \
  -o hello hello.o 2>&1 | head -20
STATUS=$?
set -e
echo "link exit: $STATUS"
[ -f hello ] && file hello
