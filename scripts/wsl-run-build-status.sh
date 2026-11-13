#!/usr/bin/env bash
# Run the build and report the exact exit status plus the last lines of output.
set -uo pipefail
bash /mnt/d/ios2/liner-music-ios/scripts/ios-build.sh > /tmp/ios-build.log 2>&1
status=$?
echo "EXIT_STATUS=${status}"
echo "--- last 8 lines ---"
tail -8 /tmp/ios-build.log
