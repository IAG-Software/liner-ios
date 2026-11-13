#!/usr/bin/env bash
set -euo pipefail
R=/opt/rust/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f
sed -n '612,660p' "$R/tauri-2.12.5/src/plugin.rs"
echo "=== macro panics ==="
grep -rn "panic!" "$R/tauri-macros-2.7.4/src/" | head -20
