#!/usr/bin/env bash
set -uo pipefail
R=/opt/rust/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f

echo "=== how codegen decides dev vs embedded assets ==="
grep -rn "custom-protocol\|custom_protocol" "$R/tauri-codegen-2.7.4/src/" | head -10
echo
grep -rn "fn is_dev\|dev_url\|cfg!(dev)" "$R/tauri-codegen-2.7.4/src/context.rs" | head -15
echo
echo "=== tauri-build: what sets cfg(dev) ==="
grep -rn "rustc-cfg=dev\|is_dev\|custom-protocol" "$R/tauri-build-2.7.4/src/lib.rs" | head -15
