#!/usr/bin/env bash
set -euo pipefail
R=$(ls -d /opt/rust/.cargo/registry/src/*/ 2>/dev/null | head -1)
echo "registry: $R"
TAURI=$(ls -d "$R"tauri-2.12.5 2>/dev/null || true)
echo "tauri: $TAURI"
echo "=== register_uri_scheme_protocol ==="
grep -rn "register_uri_scheme_protocol" "$TAURI/src/" | head -5
echo
echo "=== handler type alias ==="
grep -rn "UriSchemeResponder\|UriSchemeProtocol" "$TAURI/src/" | head -12
echo
echo "=== macro underscore panic ==="
MAC=$(ls -d "$R"tauri-macros-2.7.4 2>/dev/null || true)
grep -rn "underscore\|starts_with('_')\|is_empty()" "$MAC/src/" 2>/dev/null | head -10
