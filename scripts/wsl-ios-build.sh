#!/usr/bin/env bash
# Cross-compile Liner Music's Rust core for aarch64-apple-ios on a Linux/WSL
# host. Mirrors what xtool sets up: an iPhoneOS SDK, clang as the C driver for
# the target triple, and rust-lld as the link driver.
set -euo pipefail

export RUSTUP_HOME=/opt/rust/.rustup
export CARGO_HOME=/opt/rust/.cargo
export PATH="$CARGO_HOME/bin:$PATH"

SDKROOT=${SDKROOT:-/opt/ios/sdk-src/iPhoneOS16.5.sdk}
DEPLOYMENT_TARGET=${IPHONEOS_DEPLOYMENT_TARGET:-16.5}
TARGET=aarch64-apple-ios
SRC=${SRC:-/opt/ios/src}
MODE=${1:-check}

export SDKROOT
export IPHONEOS_DEPLOYMENT_TARGET="$DEPLOYMENT_TARGET"

TRIPLE_FLAGS="-target arm64-apple-ios${DEPLOYMENT_TARGET} -isysroot ${SDKROOT} -arch arm64"
export CC_aarch64_apple_ios="clang ${TRIPLE_FLAGS}"
export CXX_aarch64_apple_ios="clang++ ${TRIPLE_FLAGS}"
export AR_aarch64_apple_ios="llvm-ar"
export CFLAGS_aarch64_apple_ios="-O2 -fPIC -DNDEBUG"
export CARGO_TARGET_AARCH64_APPLE_IOS_LINKER="clang"

# rust-lld is shipped with the toolchain and speaks Mach-O; using it directly
# avoids needing an ld64 port on the build host.
export RUSTFLAGS="-C link-arg=-target -C link-arg=arm64-apple-ios${DEPLOYMENT_TARGET} -C link-arg=-isysroot -C link-arg=${SDKROOT}"

# Keep the build tree on the Linux filesystem: cargo over /mnt/d is ~10x slower.
mkdir -p /opt/ios/src
rsync -a --delete \
  --exclude node_modules \
  --exclude .git \
  --exclude src-tauri/target \
  /mnt/d/ios2/liner-music-ios/ /opt/ios/src/

cd /opt/ios/src/src-tauri

echo "=== cargo $MODE for $TARGET (SDK $SDKROOT, min $DEPLOYMENT_TARGET) ==="
if [ "$MODE" = "build" ]; then
  cargo build --release --target "$TARGET" --message-format short 2>&1 | tail -60
else
  cargo check --target "$TARGET" --message-format short 2>&1 | tail -80
fi
