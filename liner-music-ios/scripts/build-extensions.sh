#!/usr/bin/env bash
#
# Builds the app-extension bundles and drops them into PlugIns/.
# Sourced by ios-build.sh (it relies on $TRIPLE, $SDKROOT, $APP_DIR, $PROJECT).

build_appex() {
  local name="$1"
  local src_dir="$2"
  local frameworks="$3"

  local out="$APP_DIR/PlugIns/$name.appex"
  rm -rf "$out"
  mkdir -p "$out"

  local objects=()
  for source in "$src_dir"/*.m; do
    local object="$BUILD_DIR/obj/$name-$(basename "$source" .m).o"
    clang $TRIPLE -O2 -fobjc-arc -fmodules \
      -isystem "$SDKROOT/usr/include" \
      -c "$source" -o "$object" || return 1
    objects+=("$object")
  done

  # App extensions are entered through Foundation's `NSExtensionMain`, not
  # `main()` — that entry point is what instantiates NSExtensionPrincipalClass.
  clang $TRIPLE -fuse-ld=lld -fobjc-arc -O2 \
    -Wl,-e,_NSExtensionMain \
    -Wl,-rpath,/usr/lib/swift \
    -o "$out/$name" \
    "${objects[@]}" \
    -framework Foundation \
    -framework UIKit \
    $frameworks || return 1

  cp "$src_dir/Info.plist" "$out/Info.plist"
  printf 'XPC!????' > "$out/PkgInfo"
  echo "    $name.appex ($(du -h "$out/$name" | cut -f1))"
}

build_extensions() {
  echo "==> extensions"
  mkdir -p "$APP_DIR/PlugIns"

  build_appex "LinerWidget" "$PROJECT/ios/LinerWidget" "-framework NotificationCenter"
  build_appex "LinerMessages" "$PROJECT/ios/LinerMessages" "-framework Messages"
  build_appex "LinerIntents" "$PROJECT/ios/LinerIntents" "-framework Intents"
}
