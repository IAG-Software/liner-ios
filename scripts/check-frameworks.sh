#!/usr/bin/env bash
set -uo pipefail
S=/opt/ios/sdk-src/iPhoneOS16.5.sdk/System/Library/Frameworks
for f in GroupActivities Messages CarPlay Intents AppIntents ActivityKit AVRoutePickerView MediaPlayer; do
  if [ -d "$S/$f.framework" ]; then
    printf "%-18s present\n" "$f"
  else
    printf "%-18s MISSING\n" "$f"
  fi
done
