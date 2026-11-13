#!/usr/bin/env bash
# Verify the assembled .ipa: Mach-O sanity, plist, embedded assets, frameworks.
set -uo pipefail

APP=/opt/ios/src/build-ios/Payload/LinerMusic.app
IPA=/opt/ios/src/build-ios/LinerMusic-unsigned-ios.ipa

echo "=== ipa ==="
ls -lh "$IPA"

echo
echo "=== binary ==="
file "$APP/LinerMusic"
echo "size: $(du -h "$APP/LinerMusic" | cut -f1)"
echo "arch:"
llvm-objdump --macho --private-headers "$APP/LinerMusic" 2>/dev/null | grep -A4 "LC_BUILD_VERSION" | head -12

echo
echo "=== architecture / load commands ==="
llvm-objdump --macho --private-headers "$APP/LinerMusic" 2>/dev/null \
  | grep -E "MH_MAGIC|MH_EXECUTE|LC_BUILD_VERSION|platform|minos|sdk |LC_LOAD_DYLIB|name " | head -30

echo
echo "=== start_app exported (Tauri entry point) ==="
llvm-nm --defined-only "$APP/LinerMusic" 2>/dev/null | grep -E "_start_app|_main$" | head -5

echo
echo "=== embedded frontend (tauri-codegen) ==="
strings -a "$APP/LinerMusic" | grep -c "liner-local" || true
strings -a "$APP/LinerMusic" | grep -m3 -E "index\.html|GlobalTooltip" || true

echo
echo "=== Info.plist ==="
python3 - "$APP/Info.plist" <<'PY'
import plistlib, sys
with open(sys.argv[1], "rb") as fh:
    plist = plistlib.load(fh)
for key in ("CFBundleIdentifier", "CFBundleExecutable", "CFBundleShortVersionString",
            "CFBundleVersion", "UIBackgroundModes", "MinimumOSVersion",
            "CFBundleIcons", "UIFileSharingEnabled"):
    print(f"  {key:28s} {plist.get(key, '<absent>')}")
PY

echo
echo "=== code signature present? ==="
if llvm-objdump --macho --private-headers "$APP/LinerMusic" 2>/dev/null | grep -q LC_CODE_SIGNATURE; then
  echo "  LC_CODE_SIGNATURE present (ad-hoc or developer signed)"
else
  echo "  no LC_CODE_SIGNATURE — cleanly unsigned (expected)"
fi

echo
echo "=== frameworks referenced ==="
llvm-objdump --macho --dylibs-used "$APP/LinerMusic" 2>/dev/null | grep -oE "/System/Library/Frameworks/[A-Za-z]+\.framework" | sort -u | head -25

echo
echo "=== zip integrity ==="
unzip -t "$IPA" >/dev/null 2>&1 && echo "  archive OK" || echo "  ARCHIVE CORRUPT"
