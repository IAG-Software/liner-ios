#!/usr/bin/env bash
set -euo pipefail
mkdir -p /opt/ios/smoke/src

cat > /opt/ios/smoke/Cargo.toml <<'TOML'
[package]
name = "smoke"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["staticlib"]

[dependencies]
tauri = { version = "2.12", features = [] }
TOML

cat > /opt/ios/smoke/src/lib.rs <<'RS'
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {}

#[no_mangle]
pub extern "C" fn smoke_marker() -> i32 {
    42
}
RS

echo "--- smoke crate ready ---"
cat /opt/ios/smoke/Cargo.toml
