#!/usr/bin/env bash
# Install a dedicated Rust toolchain inside WSL for iOS cross-compilation.
set -euo pipefail

export RUSTUP_HOME=/opt/rust/.rustup
export CARGO_HOME=/opt/rust/.cargo
export PATH="$CARGO_HOME/bin:$PATH"

mkdir -p "$CARGO_HOME"

if [ ! -x "$CARGO_HOME/bin/rustc" ]; then
  curl -sSf https://sh.rustup.rs -o /tmp/rustup.sh
  sh /tmp/rustup.sh -y --no-modify-path --default-toolchain stable --profile minimal
fi

"$CARGO_HOME/bin/rustup" target add aarch64-apple-ios >/dev/null 2>&1 || true
"$CARGO_HOME/bin/rustup" component add rust-src >/dev/null 2>&1 || true

echo "--- versions ---"
"$CARGO_HOME/bin/rustc" --version
"$CARGO_HOME/bin/cargo" --version
"$CARGO_HOME/bin/rustup" target list --installed
