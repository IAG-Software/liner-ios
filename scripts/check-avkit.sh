#!/usr/bin/env bash
set -uo pipefail
S=/opt/ios/sdk-src/iPhoneOS16.5.sdk/System/Library/Frameworks
for f in AVKit AVFoundation MediaPlayer; do
  if [ -d "$S/$f.framework" ]; then printf "%-16s present\n" "$f"; else printf "%-16s MISSING\n" "$f"; fi
done
echo "--- AVRoutePickerView header? ---"
ls "$S/AVKit.framework/Headers/AVRoutePickerView.h" 2>/dev/null || echo "no header"
