fn main() {
    // Compile the iOS platform shim into the crate itself.
    //
    // Why: the crate builds three artefacts (`staticlib`, `cdylib`, `rlib`).
    // Cargo *links* the cdylib, and the linker must resolve the `liner_*`
    // externs declared in `src/platform_ios.rs`. Compiling the shim here makes
    // those symbols part of the archive, so every artefact links — and the final
    // .app link no longer needs a separate `LinerPlatform.o`.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("ios") {
        let mut build = cc::Build::new();
        build
            .file("../ios/LinerMusic/LinerPlatform.m")
            .file("../ios/LinerMusic/LinerTauriSwiftShim.m")
            .flag("-fobjc-arc")
            .flag("-fno-objc-arc-exceptions")
            .flag_if_supported("-Wno-deprecated-declarations")
            .warnings(false);

        // `cc` picks up CC_aarch64_apple_ios / CFLAGS_aarch64_apple_ios from the
        // environment, which the build script sets to the iPhoneOS SDK.
        build.compile("liner-platform");

        println!("cargo:rerun-if-changed=../ios/LinerMusic/LinerPlatform.m");
        println!("cargo:rerun-if-changed=../ios/LinerMusic/LinerTauriSwiftShim.m");
    }

    tauri_build::build()
}
