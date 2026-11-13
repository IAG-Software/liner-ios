#!/usr/bin/env bash
# Iterate on `cargo check` for the iOS target, showing full diagnostics.
set -euo pipefail

export RUSTUP_HOME=/opt/rust/.rustup
export CARGO_HOME=/opt/rust/.cargo
export PATH="$CARGO_HOME/bin:$PATH"

SDKROOT=${SDKROOT:-/opt/ios/sdk-src/iPhoneOS16.5.sdk}
DEPLOYMENT_TARGET=${IPHONEOS_DEPLOYMENT_TARGET:-16.5}
export SDKROOT IPHONEOS_DEPLOYMENT_TARGET="$DEPLOYMENT_TARGET"

TRIPLE_FLAGS="-target arm64-apple-ios${DEPLOYMENT_TARGET} -isysroot ${SDKROOT} -arch arm64"
export CC_aarch64_apple_ios="clang ${TRIPLE_FLAGS}"
export CXX_aarch64_apple_ios="clang++ ${TRIPLE_FLAGS}"
export AR_aarch64_apple_ios="llvm-ar"
export CFLAGS_aarch64_apple_ios="-O2 -fPIC -DNDEBUG"
export RUSTFLAGS="-C link-arg=-target -C link-arg=arm64-apple-ios${DEPLOYMENT_TARGET} -C link-arg=-isysroot -C link-arg=-isysroot -C link-arg=${SDKROOT}"

rsync -a --delete --exclude node_modules --exclude .git --exclude target \
  /mnt/d/ios2/liner-music-ios/ /opt/ios/src/

cd /opt/ios/src/src-tauri
set +e
cargo check --target aarch64-apple-ios 2>&1 | grep -v "^    Checking\|^   Compiling\|^warning: unused\|^ *|" | head -120
