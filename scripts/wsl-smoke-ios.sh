#!/usr/bin/env bash
# Smoke test: can Tauri v2 build for aarch64-apple-ios with the xtool-style
# clang + iPhoneOS SDK layout?
set -euo pipefail

export RUSTUP_HOME=/opt/rust/.rustup
export CARGO_HOME=/opt/rust/.cargo
export PATH="$CARGO_HOME/bin:$PATH"

SDKROOT=${SDKROOT:-/opt/ios/sdk-src/iPhoneOS16.5.sdk}
export SDKROOT
export IPHONEOS_DEPLOYMENT_TARGET=${IPHONEOS_DEPLOYMENT_TARGET:-16.5}

CFLAGS_TARGET="-target arm64-apple-ios${IPHONEOS_DEPLOYMENT_TARGET} -isysroot ${SDKROOT} -arch arm64"
export CC_aarch64_apple_ios="clang ${CFLAGS_TARGET}"
export CXX_aarch64_apple_ios="clang++ ${CFLAGS_TARGET}"
export AR_aarch64_apple_ios="llvm-ar"
export CFLAGS_aarch64_apple_ios="-O2 -fPIC"
export CARGO_TARGET_AARCH64_APPLE_IOS_LINKER="clang"
export RUSTFLAGS="-C link-arg=-target -C link-arg=arm64-apple-ios${IPHONEOS_DEPLOYMENT_TARGET} -C link-arg=-isysroot -C link-arg=${SDKROOT}"

cd /opt/ios/smoke
cargo check --target aarch64-apple-ios --message-format short 2>&1 | tail -40
