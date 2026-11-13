#!/usr/bin/env bash
set -uo pipefail
IPA=/opt/ios/src/build-ios/LinerMusic-unsigned-ios.ipa
echo "=== payload tree ==="
unzip -l "$IPA" | awk '{print $4}' | grep -v '^$' | sed 's|Payload/LinerMusic.app/||' | sort | head -40
echo
echo "=== appex bundles ==="
unzip -l "$IPA" | grep -c "appex/"
echo
echo "=== URL scheme in plist ==="
unzip -p "$IPA" Payload/LinerMusic.app/Info.plist | grep -A2 CFBundleURLSchemes | head -6
