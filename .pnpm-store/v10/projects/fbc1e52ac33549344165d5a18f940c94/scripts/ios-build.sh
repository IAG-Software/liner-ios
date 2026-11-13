#!/usr/bin/env bash
#
# Liner Music — Windows/Linux → iOS (arm64) cross-compilation and unsigned .ipa
# assembly.
#
# This performs exactly what xtool does for a Swift package, for a Rust binary:
#   1. compile the Rust staticlib for aarch64-apple-ios
#   2. compile the ObjC host with clang against the iPhoneOS SDK
#   3. link with lld (Mach-O) against the SDK's .tbd stubs
#   4. assemble Payload/LinerMusic.app, ad-hoc sign, zip
#   5. emit LinerMusic-unsigned-ios.ipa
#
# No Xcode, no macOS, no Apple developer certificate.
#
# Usage:  bash scripts/ios-build.sh
set -euo pipefail

# ---------------------------------------------------------------- config ----
export RUSTUP_HOME=${RUSTUP_HOME:-/opt/rust/.rustup}
export CARGO_HOME=${CARGO_HOME:-/opt/rust/.cargo}
export PATH="$CARGO_HOME/bin:$PATH"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SRC_ROOT="${SRC_ROOT:-$(cd "$SCRIPT_DIR/.." && pwd)}"   # project checkout
PROJECT=${PROJECT:-/opt/ios/src}                        # ext4 build copy
SDKROOT=${SDKROOT:-/opt/ios/sdk-src/iPhoneOS16.5.sdk}
DEPLOYMENT_TARGET=${IPHONEOS_DEPLOYMENT_TARGET:-16.5}
TARGET=aarch64-apple-ios
APP_NAME=LinerMusic
BUNDLE_ID=com.iagsoftware.linermusic
BUILD_DIR="$PROJECT/build-ios"
APP_DIR="$BUILD_DIR/Payload/$APP_NAME.app"
LIB="$PROJECT/src-tauri/target/$TARGET/release/libliner_music_lib.a"

mkdir -p "$BUILD_DIR"

# App extensions (Today widget, iMessage, Intents) live in PlugIns/.
source "$SCRIPT_DIR/build-extensions.sh"

# --------------------------------------------------------------- toolchain ---
export SDKROOT IPHONEOS_DEPLOYMENT_TARGET="$DEPLOYMENT_TARGET"
TRIPLE="-target arm64-apple-ios${DEPLOYMENT_TARGET} -isysroot ${SDKROOT} -arch arm64"
export CC_aarch64_apple_ios="clang ${TRIPLE}"
export CXX_aarch64_apple_ios="clang++ ${TRIPLE}"
export AR_aarch64_apple_ios="llvm-ar"
export CFLAGS_aarch64_apple_ios="-O2 -fPIC -DNDEBUG"
export RUSTFLAGS="-C link-arg=-target -C link-arg=arm64-apple-ios${DEPLOYMENT_TARGET} -C link-arg=-isysroot -C link-arg=${SDKROOT}"

# rustc invokes `$CC` as the link driver. The host clang defaults to binutils
# `ld` (ELF only), so route it through lld's Mach-O flavour explicitly.
mkdir -p /opt/ios/bin
cat > /opt/ios/bin/ios-linker <<'WRAPPER'
#!/usr/bin/env bash
exec clang -fuse-ld=lld "$@"
WRAPPER
chmod +x /opt/ios/bin/ios-linker
export CARGO_TARGET_AARCH64_APPLE_IOS_LINKER=/opt/ios/bin/ios-linker

echo "==> [1/6] sync sources → $PROJECT"
# cargo and clang are ~10x faster on ext4 than on the 9p mount that exposes
# /mnt/d inside WSL.
mkdir -p "$PROJECT"
rsync -a --delete \
  --exclude node_modules \
  --exclude .git \
  --exclude target \
  --exclude build-ios \
  --exclude release \
  "$SRC_ROOT"/ "$PROJECT"/

if [ ! -f "$PROJECT/dist/index.html" ]; then
  echo "!! dist/index.html missing — run 'pnpm build' on the host first" >&2
  exit 1
fi

echo "==> [2/6] cargo build ($TARGET, release)"
cd "$PROJECT/src-tauri"
# `custom-protocol` is what makes Tauri serve the assets embedded by
# `generate_context!` instead of pointing WKWebView at `build.devUrl`
# (http://localhost:5173). The Tauri CLI passes this automatically for
# `tauri build`; we drive cargo directly, so it must be explicit here.
# Without it the app boots into Tauri's "development server unreachable" page.
cargo build --release --target "$TARGET" --features custom-protocol
if [ ! -f "$LIB" ]; then
  echo "!! staticlib not produced at $LIB" >&2
  exit 1
fi
echo "    libliner_music_lib.a: $(du -h "$LIB" | cut -f1)"

echo "==> [3/6] clang: host objects"
mkdir -p "$BUILD_DIR/obj"
# LinerPlatform.m and LinerTauriSwiftShim.m are compiled *into* the Rust archive
# by src-tauri/build.rs (cc), so both the staticlib and the cdylib link. Only
# the entry point is built here.
clang $TRIPLE -O2 -fobjc-arc -fmodules -std=c++17 \
  -isystem "$SDKROOT/usr/include" \
  -c "$PROJECT/ios/$APP_NAME/main.mm" -o "$BUILD_DIR/obj/main.o"

echo "==> [4/6] link ($APP_NAME)"
rm -rf "$APP_DIR"
mkdir -p "$APP_DIR"

# Tauri's iOS runtime is partly Swift (wry's WKWebView bridge), so the Swift
# runtime stubs from the SDK are needed at link time. iOS ships the runtime from
# 12.2 onwards, so nothing is embedded in the bundle.
SWIFT_LIBS=()
if [ -d "$SDKROOT/usr/lib/swift" ]; then
  SWIFT_LIBS+=("-L$SDKROOT/usr/lib/swift")
  for lib in swiftCore swiftFoundation swiftDispatch swiftObjectiveC swift_Concurrency swiftUniformTypeIdentifiers; do
    if [ -f "$SDKROOT/usr/lib/swift/lib${lib}.tbd" ]; then
      SWIFT_LIBS+=("-l${lib}")
    fi
  done
fi

clang $TRIPLE -fuse-ld=lld -fobjc-arc -O2 \
  -Wl,-platform_version -Wl,ios,"$DEPLOYMENT_TARGET","$DEPLOYMENT_TARGET" \
  -Wl,-rpath,/usr/lib/swift \
  -o "$APP_DIR/$APP_NAME" \
  "$BUILD_DIR/obj/main.o" \
  -Wl,-force_load,"$LIB" \
  -framework UIKit \
  -framework Foundation \
  -framework CoreFoundation \
  -framework WebKit \
  -framework JavaScriptCore \
  -framework AVFoundation \
  -framework AVKit \
  -framework MediaPlayer \
  -framework CoreMedia \
  -framework CoreGraphics \
  -framework CoreText \
  -framework QuartzCore \
  -framework Metal \
  -framework MetalKit \
  -framework Security \
  -framework AudioToolbox \
  -framework UniformTypeIdentifiers \
  -framework CoreServices \
  -framework SystemConfiguration \
  -lz -lc++ -liconv -lsqlite3 \
  "${SWIFT_LIBS[@]}"

echo "    $(du -h "$APP_DIR/$APP_NAME" | cut -f1)  $APP_NAME"

# Assert the app is a *production* build: with `custom-protocol` off, Tauri
# embeds an empty asset table and points WKWebView at build.devUrl, which on a
# device renders "Failed to request http://localhost:5173/". Failing here is far
# cheaper than discovering that on the phone.
#
# `grep -a` on the file rather than piping `llvm-nm` into `grep -q`: the latter
# trips `pipefail` when grep closes the pipe early.
if ! grep -aq "tauri_utils6assets14EmbeddedAssets" "$APP_DIR/$APP_NAME"; then
  echo "!! binary has no embedded assets — it was built without" >&2
  echo "   --features custom-protocol and would load the dev-server URL" >&2
  exit 1
fi
echo "    embedded assets: present"

build_extensions

echo "==> [5/6] bundle"
cp "$PROJECT/ios/$APP_NAME/Info.plist" "$APP_DIR/Info.plist"
VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' "$PROJECT/src-tauri/Cargo.toml" | head -1)
sed -i "s|<string>0.2.12</string>|<string>${VERSION}</string>|" "$APP_DIR/Info.plist"
# The plist ships a default MinimumOSVersion; keep it in lockstep with the
# deployment target actually used to link.
sed -i "s|<key>MinimumOSVersion</key>[[:space:]]*<string>[^<]*</string>|<key>MinimumOSVersion</key><string>${DEPLOYMENT_TARGET}</string>|" \
  "$APP_DIR/Info.plist"

printf 'APPL????' > "$APP_DIR/PkgInfo"

if [ -d "$PROJECT/ios/$APP_NAME/icons" ]; then
  cp "$PROJECT/ios/$APP_NAME/icons/"*.png "$APP_DIR/" 2>/dev/null || true
fi

# The frontend is embedded in the binary by tauri-codegen (generate_context!
# reads `frontendDist` at compile time), so no second copy of dist/ ships.
echo "    frontend embedded in binary"

# Ad-hoc signature: sideloading tools (SideStore/AltStore/TrollStore) re-sign
# anyway, but an ad-hoc signature keeps the bundle inspectable and lets
# TrollStore install it directly.
if command -v ldid >/dev/null 2>&1; then
  echo "    ad-hoc signing with ldid"
  ldid -S "$APP_DIR/$APP_NAME"
else
  echo "    ldid not found — leaving the binary unsigned (expected for sideloading)"
fi

echo "==> [6/6] package"
cd "$BUILD_DIR"
rm -f LinerMusic-unsigned-ios.ipa
# `zip -X` strips extra attributes so the archive is byte-stable.
zip -qr -X LinerMusic-unsigned-ios.ipa Payload

echo
echo "=== done ==="
ls -lh LinerMusic-unsigned-ios.ipa
echo "payload:"
unzip -l LinerMusic-unsigned-ios.ipa | sed -n '1,22p' || true
