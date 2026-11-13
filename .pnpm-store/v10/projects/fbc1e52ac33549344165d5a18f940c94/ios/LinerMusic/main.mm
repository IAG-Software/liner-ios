// Liner Music — iOS entry point.
//
// This is the exact shape Tauri v2 expects: the Rust static library exports
// `start_app`, which installs the Tauri runtime (UIApplication + WKWebView) and
// does not return until the app terminates. `main` therefore has no Autorelease
// Pool of its own to drain afterwards.
//
// Reference: tauri/crates/tauri-cli/templates/mobile/ios/Sources/{{app.name}}/main.mm

#include <stdint.h>

extern "C" {
    void start_app(void);
    // Defined in LinerPlatform.m; appends to Documents/liner-boot.log so a
    // launch failure is diagnosable on-device.
    void liner_boot_log(const char *message);
}

int main(int argc, char *argv[]) {
    (void)argc;
    (void)argv;

    liner_boot_log("main: entered");
    start_app();
    // Reached only if the runtime event loop returned (i.e. the app is
    // terminating).
    liner_boot_log("main: start_app returned");
    return 0;
}
