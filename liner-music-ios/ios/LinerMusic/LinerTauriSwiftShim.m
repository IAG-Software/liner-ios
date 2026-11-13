// Liner Music — C implementation of the four symbols Tauri's iOS runtime
// expects from its Swift API package (`tauri/mobile/ios-api`).
//
// WHY THIS EXISTS
// ---------------
// `tauri::ios` declares (via `swift!`):
//
//     run_plugin_command(id, name, method, data, callback, channel_cb)
//     register_plugin(name, plugin, config, webview)
//     on_webview_created(webview, controller)
//     log_stdout()
//
// plus the `swift-rs` object shims `retain_object` / `release_object` /
// `string_from_bytes`. In an Xcode build those come from the
// `tauri/mobile/ios-api` Swift package, compiled by `swiftc`.
//
// This port deliberately registers **no mobile Swift plugins**. Every command
// the renderer uses is a Rust `#[tauri::command]` (see `src/commands/`), so the
// only one of these that is actually invoked is `on_webview_created`, which is
// a hook for plugins to attach to the WKWebView — with no plugins it is a
// no-op by definition.
//
// The `swift-rs` symbols would only be reached while marshalling arguments into
// a Swift plugin call; that path is unreachable here.
//
// If you later add a Swift-based plugin (anything depending on
// `swift-rs`/`tauri::plugin::mobile`), delete this file and build
// `tauri/mobile/ios-api` with a Swift cross-toolchain instead — see
// `docs/PORTING-NOTES.md` §10.

#import <Foundation/Foundation.h>
#import <UIKit/UIKit.h>
#import <WebKit/WebKit.h>

extern void liner_boot_log(const char *message);

// Keep a reference so a future plugin host can pick the webview up.
static void *gLastWebview = NULL;
static void *gLastController = NULL;

#pragma mark - Tauri iOS API

/// Called once per webview creation (tauri/src/manager/webview.rs).
///
/// This is where the inline-media policy is applied. `allowsInlineMediaPlayback`
/// must be on or WKWebView forces fullscreen for every `<video>`/`<audio>`; and
/// clearing `mediaTypesRequiringUserActionForPlayback` lets a `<audio>` element
/// start without a second tap (the native engine already owns real playback, this
/// is for the renderer's WebAudio/visualiser fallback paths).
void on_webview_created(const void *webview, const void *controller) {
    gLastWebview = (void *)webview;
    gLastController = (void *)controller;
    liner_boot_log("webview: created");

    if (webview == NULL) {
        liner_boot_log("webview: NULL pointer");
        return;
    }

    WKWebView *webView = (__bridge WKWebView *)gLastWebview;
    WKWebViewConfiguration *configuration = webView.configuration;

    configuration.allowsInlineMediaPlayback = YES;
    if (@available(iOS 10.0, *)) {
        configuration.mediaTypesRequiringUserActionForPlayback = WKAudiovisualMediaTypeNone;
        configuration.allowsPictureInPictureMediaPlayback = YES;
    }

    // A colour that cannot be confused with "no window at all": if the screen is
    // this shade, the webview exists and the failure is in the page.
    UIColor *backdrop = [UIColor colorWithRed:0.07 green:0.09 blue:0.13 alpha:1.0];
    webView.opaque = YES;
    webView.backgroundColor = backdrop;
    webView.scrollView.backgroundColor = backdrop;
}

/// Not called by this app: we install no mobile plugins.
void register_plugin(const char *name,
                     const void *plugin,
                     const char *config,
                     const void *webview) {
    (void)plugin;
    (void)webview;
    NSLog(@"[liner] register_plugin(%s) ignored — no mobile plugins are registered",
          name ? name : "");
}

/// Not called by this app: we install no mobile plugins.
void run_plugin_command(int32_t identifier,
                        const void *name,
                        const void *method,
                        const void *data,
                        void *callback,
                        void *send_channel_data_callback) {
    (void)identifier;
    (void)name;
    (void)method;
    (void)data;
    (void)callback;
    (void)send_channel_data_callback;
}

/// On iOS stdout is not attached to anything a developer can see; Tauri calls
/// this to mirror it into the system log. Nothing writes to stdout here, so a
/// no-op is correct.
void log_stdout(void) {}

#pragma mark - swift-rs object shims
//
// These are the raw Swift-runtime entry points `swift-rs` links against. They
// are only reachable through `swift!` calls that pass `SRString`/`SRData`
// arguments — i.e. plugin commands, which do not exist in this build. The
// definitions below keep the linker satisfied and fail loudly if that
// assumption is ever violated at runtime.

void *string_from_bytes(const char *bytes, size_t length) {
    (void)bytes;
    (void)length;
    NSLog(@"[liner] FATAL: swift-rs string_from_bytes called but the Tauri Swift "
          @"API package is not linked (no Swift plugins supported in this build)");
    return NULL;
}

void *data_from_bytes(const char *bytes, size_t length) {
    (void)bytes;
    (void)length;
    NSLog(@"[liner] FATAL: swift-rs data_from_bytes called but the Tauri Swift "
          @"API package is not linked (no Swift plugins supported in this build)");
    return NULL;
}

void *retain_object(const void *object) {
    return (void *)object;
}

void release_object(const void *object) {
    (void)object;
}
