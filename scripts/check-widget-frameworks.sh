#!/usr/bin/env bash
set -uo pipefail
S=/opt/ios/sdk-src/iPhoneOS16.5.sdk/System/Library/Frameworks
for f in WidgetKit SwiftUI ActivityKit AppIntents Intents GroupActivities CarPlay Charts TipKit StoreKit; do
  if [ -d "$S/$f.framework" ]; then printf "%-16s present\n" "$f"; else printf "%-16s MISSING\n" "$f"; fi
done
echo "--- widget extension point supported by the OS? (info only) ---"
echo "com.apple.widgetkit-extension requires a WidgetKit target (SwiftUI)"
