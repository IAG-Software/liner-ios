#!/usr/bin/env bash
set -euo pipefail
python3 - <<'PY'
import plistlib

path = "/opt/ios/src/build-ios/Payload/LinerMusic.app/Info.plist"
with open(path, "rb") as fh:
    plist = plistlib.load(fh)

keys = [
    "CFBundleIdentifier",
    "CFBundleExecutable",
    "CFBundleShortVersionString",
    "CFBundleVersion",
    "MinimumOSVersion",
    "UIDeviceFamily",
    "CFBundleSupportedPlatforms",
    "UIBackgroundModes",
    "UIFileSharingEnabled",
    "MinimumOSVersion",
]
for key in keys:
    print(f"  {key:28s} {plist.get(key, '<absent>')}")
PY
