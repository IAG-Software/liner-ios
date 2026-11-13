from pathlib import Path

# ---- 1. hook extensions into the build -------------------------------------
p = Path(r"D:\ios2\liner-music-ios\scripts\ios-build.sh")
s = p.read_text(encoding="utf-8")
if "build-extensions.sh" not in s:
    s = s.replace(
        'echo "==> [5/6] bundle"',
        '# App extensions (Today widget, iMessage, Intents) live in PlugIns/.\n'
        'source "$SCRIPT_DIR/build-extensions.sh"\n'
        '\n'
        'echo "==> [5/6] bundle"',
    )
    s = s.replace(
        'echo "    embedded assets: present"',
        'echo "    embedded assets: present"\n\nbuild_extensions',
    )
p.write_text(s, encoding="utf-8")

# ---- 2. ObjC: URL scheme hook ----------------------------------------------
p = Path(r"D:\ios2\liner-music-ios\ios\LinerMusic\LinerPlatform.m")
s = p.read_text(encoding="utf-8")
if "liner_url_install" not in s:
    s = s.replace('#import <UniformTypeIdentifiers/UniformTypeIdentifiers.h>',
                  '#import <UniformTypeIdentifiers/UniformTypeIdentifiers.h>\n#import <objc/runtime.h>')
    s += '''
#pragma mark - Deep links (liner://)

static void (*gUrlCallback)(const char *) = NULL;

/// Injected onto whatever delegate class Tauri created. Returns YES so the
/// system does not fall back to opening the URL externally.
static BOOL LinerOpenURL(id self, SEL _cmd, UIApplication *app, NSURL *url, NSDictionary *options) {
    (void)self; (void)_cmd; (void)app; (void)options;
    if (gUrlCallback != NULL && url.absoluteString.length > 0) {
        gUrlCallback(url.absoluteString.UTF8String);
    }
    return YES;
}

/// The UIKit delegate is owned by Tauri's runtime, so the handler is added to
/// its class at runtime rather than requiring our own AppDelegate.
void liner_url_install(void (*callback)(const char *)) {
    gUrlCallback = callback;

    dispatch_async(dispatch_get_main_queue(), ^{
        id delegate = [UIApplication sharedApplication].delegate;
        if (delegate == nil) return;

        Class cls = object_getClass(delegate);
        SEL sel = sel_registerName("application:openURL:options:");
        Method existing = class_getInstanceMethod(cls, sel);
        if (existing != NULL) {
            method_setImplementation(existing, (IMP)LinerOpenURL);
        } else {
            class_addMethod(cls, sel, (IMP)LinerOpenURL, "B@:@@@");
        }
    });
}
'''
p.write_text(s, encoding="utf-8")

# ---- 3. Rust: extern + event -----------------------------------------------
p = Path(r"D:\ios2\liner-music-ios\src-tauri\src\platform_ios.rs")
s = p.read_text(encoding="utf-8")
if "liner_url_install" not in s:
    s = s.replace(
        "    fn liner_show_route_picker() -> c_int;",
        "    fn liner_show_route_picker() -> c_int;\n    fn liner_url_install(callback: extern \"C\" fn(*const c_char));",
    )
    s = s.replace(
        "pub mod route_picker {",
        '''/// Deep links: `liner://…` → `deep-link` event for the renderer.
pub fn install_deep_links(app: tauri::AppHandle) {
    use tauri::Emitter;

    extern "C" fn on_url(url: *const c_char) {
        if url.is_null() {
            return;
        }
        let Ok(value) = unsafe { std::ffi::CStr::from_ptr(url) }.to_str() else {
            return;
        };
        if let Some(handle) = crate::state::current_handle() {
            let _ = handle.emit("deep-link", value.to_string());
        }
    }

    let _ = &app;
    unsafe { liner_url_install(on_url) };
}

pub mod route_picker {''',
    )
p.write_text(s, encoding="utf-8")

# call it from install()
p = Path(r"D:\ios2\liner-music-ios\src-tauri\src\platform_ios.rs")
s = p.read_text(encoding="utf-8")
if "install_deep_links(app.clone())" not in s:
    s = s.replace(
        "    spawn_remote_forwarder(app.clone());",
        "    spawn_remote_forwarder(app.clone());\n    install_deep_links(app.clone());",
    )
p.write_text(s, encoding="utf-8")

# ---- 4. Bridge: expose the event -------------------------------------------
p = Path(r"D:\ios2\liner-music-ios\src\tauri\api.ts")
s = p.read_text(encoding="utf-8")
s = s.replace(
    "  onDeeplink: (cb: (target: DeeplinkTarget) => void) => () => void;",
    "  onDeeplink: (cb: (target: DeeplinkTarget) => void) => () => void;\n"
    "  /** Raw `liner://…` URL, from Siri, Shortcuts, widgets or another app. */\n"
    "  onOpenUrl?: (cb: (url: string) => void) => () => void;",
)
p.write_text(s, encoding="utf-8")

p = Path(r"D:\ios2\liner-music-ios\src\tauri\bridge.ts")
s = p.read_text(encoding="utf-8")
s = s.replace(
    "    onDeeplink: (_cb: (target: DeeplinkTarget) => void) => unsub(),",
    "    onDeeplink: (_cb: (target: DeeplinkTarget) => void) => unsub(),\n"
    "    onOpenUrl: (cb: (url: string) => void) =>\n"
    "      subscribe<string>(\"deep-link\", cb),",
)
p.write_text(s, encoding="utf-8")

print("deep links wired")
