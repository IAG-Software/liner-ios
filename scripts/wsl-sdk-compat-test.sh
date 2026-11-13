#!/usr/bin/env bash
# Experiment: seed Foundation's missing forward declarations in one central
# header (NSObjCRuntime.h), which every Foundation header includes first.
set -uo pipefail

SDK=/opt/ios/sdk-src/iPhoneOS16.5.sdk
H=$SDK/System/Library/Frameworks/Foundation.framework/Headers/NSObjCRuntime.h
cp -n "$H" "$H.orig" 2>/dev/null || true

if ! grep -q "LINER_SDK_COMPAT" "$H"; then
cat >> "$H" <<'EOF'

/* LINER_SDK_COMPAT: the public theos stub SDK omits forward declarations that
   Apple's umbrella headers assume exist (NSError in NSArray.h is the first).
   Seeding them here, in the header every Foundation header includes first,
   lets Swift's clang importer build the module. */
#if defined(__OBJC__)
@class NSError, NSData, NSMutableData, NSString, NSMutableString;
@class NSArray, NSMutableArray, NSDictionary, NSMutableDictionary;
@class NSNumber, NSValue, NSDate, NSURL, NSURLRequest, NSURLResponse;
@class NSHTTPURLResponse, NSCoder, NSObject, NSSet, NSMutableSet, NSOrderedSet;
@class NSIndexSet, NSIndexPath, NSLocale, NSTimeZone, NSCalendar, NSDateComponents;
@class NSDateFormatter, NSNumberFormatter, NSFileManager, NSFileHandle, NSBundle;
@class NSProcessInfo, NSThread, NSNotification, NSNotificationCenter;
@class NSUserDefaults, NSUUID, NSProgress, NSException, NSMethodSignature;
@class NSInvocation, NSPort, NSRunLoop, NSTimer, NSURLSession;
@class NSURLSessionConfiguration, NSURLComponents, NSURLQueryItem;
@class NSAttributedString, NSMutableAttributedString, NSCharacterSet;
@class NSDecimalNumber, NSMeasurement, NSUnit, NSTextCheckingResult;
@class NSRegularExpression, NSPredicate, NSSortDescriptor, NSExpression;
@class NSOperation, NSOperationQueue, NSLock, NSCondition, NSScanner;
@class NSStream, NSInputStream, NSOutputStream, NSPipe, NSProxy;
#endif
EOF
fi

TC=/opt/swift/swift-6.4.2-RELEASE-ubuntu24.04
WORK=/tmp/swift-smoke
mkdir -p "$WORK"; cd "$WORK"
printf 'import SwiftUI\nstruct V: View { var body: some View { Text("hi") } }\n' > ui.swift
rm -f ui.o
echo "=== compile ==="
"$TC/usr/bin/swiftc" -target arm64-apple-ios16.5 -sdk "$SDK" \
  -parse-as-library -O -emit-object -o ui.o ui.swift 2>&1 | grep -E "error:" | head -6
if [ -f ui.o ]; then echo "SWIFTUI OK"; else echo "still failing"; fi
