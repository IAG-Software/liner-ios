#!/usr/bin/env bash
set -uo pipefail
R=/opt/rust/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f

echo "=== tauri-build::is_dev ==="
sed -n '590,625p' "$R/tauri-build-2.7.4/src/lib.rs"

echo
echo "=== context.rs around 165-195 ==="
sed -n '160,200p' "$R/tauri-codegen-2.7.4/src/context.rs"
