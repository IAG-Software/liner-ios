from pathlib import Path

# ---- 1. ObjC: AirPlay route picker -----------------------------------------
p = Path(r"D:\ios2\liner-music-ios\ios\LinerMusic\LinerPlatform.m")
s = p.read_text(encoding="utf-8")

if '#import <AVKit/AVKit.h>' not in s:
    s = s.replace('#import <AVFoundation/AVFoundation.h>',
                  '#import <AVFoundation/AVFoundation.h>\n#import <AVKit/AVKit.h>')

if 'liner_show_route_picker' not in s:
    s += '''
#pragma mark - AirPlay route picker

/// Presents the system route sheet (AirPlay / Bluetooth / speakers).
///
/// `AVRoutePickerView` has no programmatic "show" API, so this puts the real
/// control in a small popover; tapping its glyph opens the system list. It is a
/// genuine system picker, not a lookalike.
int liner_show_route_picker(void) {
    __block BOOL presented = NO;
    dispatch_semaphore_t done = dispatch_semaphore_create(0);

    dispatch_async(dispatch_get_main_queue(), ^{
        UIViewController *host = LinerTopViewController();
        if (host == nil) {
            dispatch_semaphore_signal(done);
            return;
        }

        UIViewController *sheet = [[UIViewController alloc] initWithNibName:nil bundle:nil];
        sheet.view.backgroundColor = [UIColor clearColor];
        sheet.modalPresentationStyle = UIModalPresentationPopover;
        sheet.preferredContentSize = CGSizeMake(150, 84);

        AVRoutePickerView *picker = [[AVRoutePickerView alloc] initWithFrame:CGRectZero];
        picker.translatesAutoresizingMaskIntoConstraints = NO;
        picker.tintColor = [UIColor whiteColor];
        picker.activeTintColor = [UIColor systemBlueColor];
        [sheet.view addSubview:picker];

        [NSLayoutConstraint activateConstraints:@[
            [picker.centerXAnchor constraintEqualToAnchor:sheet.view.centerXAnchor],
            [picker.centerYAnchor constraintEqualToAnchor:sheet.view.centerYAnchor],
            [picker.widthAnchor constraintEqualToConstant:44],
            [picker.heightAnchor constraintEqualToConstant:44],
        ]];

        UIPopoverPresentationController *popover = sheet.popoverPresentationController;
        if (popover != nil) {
            popover.sourceView = host.view;
            popover.sourceRect = CGRectMake(host.view.bounds.size.width - 32, 64, 1, 1);
            popover.permittedArrowDirections = 0;
            popover.backgroundColor = [UIColor colorWithWhite:0.1 alpha:0.9];
        }

        [host presentViewController:sheet animated:YES completion:nil];
        presented = YES;
        dispatch_semaphore_signal(done);
    });

    dispatch_semaphore_wait(done, dispatch_time(DISPATCH_TIME_NOW, (int64_t)(2 * NSEC_PER_SEC)));
    return presented ? 1 : 0;
}
'''
p.write_text(s, encoding="utf-8")

# ---- 2. Rust: extern + module ----------------------------------------------
p = Path(r"D:\ios2\liner-music-ios\src-tauri\src\platform_ios.rs")
s = p.read_text(encoding="utf-8")
s = s.replace(
    "    fn liner_pick_audio_files(callback: extern \"C\" fn(*const c_char));",
    "    fn liner_pick_audio_files(callback: extern \"C\" fn(*const c_char));\n"
    "    fn liner_show_route_picker() -> c_int;",
)
if "pub mod route_picker" not in s:
    s = s.replace(
        "/// Open a URL in the system browser / handler app.",
        """pub mod route_picker {
    //! System AirPlay / Bluetooth route sheet.

    use super::*;

    pub fn show() -> bool {
        unsafe { liner_show_route_picker() == 1 }
    }
}

/// Open a URL in the system browser / handler app.""",
    )
p.write_text(s, encoding="utf-8")

# ---- 3. Rust command ------------------------------------------------------
Path(r"D:\ios2\liner-music-ios\src-tauri\src\commands\media_cmds.rs").write_text(
    '''//! `media:*` — outputs and playback-surface controls.

use tauri::State;

use crate::state::AppState;

/// Show the system output-route sheet (AirPlay, Bluetooth, speakers).
#[tauri::command]
pub fn show_route_picker(_state: State<'_, AppState>) -> bool {
    #[cfg(target_os = "ios")]
    {
        return crate::platform_ios::route_picker::show();
    }
    #[cfg(not(target_os = "ios"))]
    {
        false
    }
}
''',
    encoding="utf-8",
)

p = Path(r"D:\ios2\liner-music-ios\src-tauri\src\commands\mod.rs")
s = p.read_text(encoding="utf-8")
if "pub mod media_cmds;" not in s:
    s = s.replace("pub mod net_cmds;", "pub mod media_cmds;\npub mod net_cmds;")
p.write_text(s, encoding="utf-8")

p = Path(r"D:\ios2\liner-music-ios\src-tauri\src\lib.rs")
s = p.read_text(encoding="utf-8")
if "media_cmds::show_route_picker" not in s:
    s = s.replace(
        "            commands::now_playing_cmds::now_playing_clear,",
        "            commands::now_playing_cmds::now_playing_clear,\n"
        "            commands::media_cmds::show_route_picker,",
    )
p.write_text(s, encoding="utf-8")

# ---- 4. JS bridge ---------------------------------------------------------
p = Path(r"D:\ios2\liner-music-ios\src\tauri\api.ts")
s = p.read_text(encoding="utf-8")
s = s.replace(
    "  onRemoteCommand?: (cb: (command: string) => void) => () => void;",
    "  /** System AirPlay / Bluetooth route sheet. */\n"
    "  showRoutePicker?: () => Promise<boolean>;\n"
    "  onRemoteCommand?: (cb: (command: string) => void) => () => void;",
)
p.write_text(s, encoding="utf-8")

p = Path(r"D:\ios2\liner-music-ios\src\tauri\bridge.ts")
s = p.read_text(encoding="utf-8")
s = s.replace(
    "    onRemoteCommand: (cb: (command: string) => void) =>",
    "    showRoutePicker: () => invoke<boolean>(\"show_route_picker\"),\n"
    "    onRemoteCommand: (cb: (command: string) => void) =>",
)
p.write_text(s, encoding="utf-8")

print("airplay wired")
