// Liner Music — iOS platform shim.
//
// Bridges the nine operations the Rust core needs from Apple's media stack.
// Compiled by the same clang invocation that links libliner_music_lib.a, so
// there is no Swift toolchain, no codegen, and no Objective-C ownership that
// has to be tracked from Rust.
//
// Ownership rules at this boundary:
//   * every `const char *` argument is borrowed for the duration of the call
//     and is copied into an NSString immediately;
//   * the two callbacks (`liner_remote_commands_install`,
//     `liner_pick_audio_files`) are invoked on the main thread;
//   * nothing here holds a reference to a Rust type.

#import <Foundation/Foundation.h>
#import <UIKit/UIKit.h>
#import <AVFoundation/AVFoundation.h>
#import <AVKit/AVKit.h>
#import <MediaPlayer/MediaPlayer.h>
#import <UniformTypeIdentifiers/UniformTypeIdentifiers.h>
#import <objc/runtime.h>

#pragma mark - Boot log

// Mirrors Rust's boot_log into the same file: <Documents>/liner-boot.log.
// UIFileSharingEnabled makes Documents visible in Files.app, which is the only
// console a sideloaded build has.
static NSString *LinerBootLogPath(void) {
    NSArray<NSString *> *directories =
        NSSearchPathForDirectoriesInDomains(NSDocumentDirectory, NSUserDomainMask, YES);
    if (directories.count == 0) {
        return nil;
    }
    return [directories.firstObject stringByAppendingPathComponent:@"liner-boot.log"];
}

void liner_boot_log(const char *message) {
    NSString *path = LinerBootLogPath();
    if (path == nil) {
        return;
    }

    NSString *line = [NSString stringWithFormat:@"[native] %s\n", (message != NULL) ? message : ""];
    NSData *payload = [line dataUsingEncoding:NSUTF8StringEncoding];
    if (payload == nil) {
        return;
    }

    NSFileHandle *handle = [NSFileHandle fileHandleForWritingAtPath:path];
    if (handle == nil) {
        [payload writeToFile:path atomically:NO];
        return;
    }
    @try {
        [handle seekToEndOfFile];
        [handle writeData:payload];
    } @catch (__unused NSException *exception) {
    } @finally {
        [handle closeFile];
    }
}

#pragma mark - Audio session

static void (*gRemoteCallback)(int) = NULL;
static double gPendingSeek = -1.0;
static BOOL gSessionObserversInstalled = NO;

static NSString *LinerString(const char *value) {
    if (value == NULL) {
        return nil;
    }
    return [NSString stringWithUTF8String:value];
}

/// `AVAudioSessionCategoryPlayback` is what makes audio survive the lock screen
/// and the silent switch. It also requires `UIBackgroundModes: [audio]` in
/// Info.plist, otherwise iOS suspends the process regardless.
int liner_audio_session_activate(void) {
    AVAudioSession *session = [AVAudioSession sharedInstance];
    NSError *error = nil;

    if (![session setCategory:AVAudioSessionCategoryPlayback
                  mode:AVAudioSessionModeDefault
               options:AVAudioSessionCategoryOptionAllowBluetoothA2DP |
                       AVAudioSessionCategoryOptionAllowAirPlay
                 error:&error]) {
        NSLog(@"[liner] setCategory failed: %@", error);
        liner_boot_log("audio_session: setCategory FAILED");
        return 1;
    }

    if (![session setActive:YES withOptions:0 error:&error]) {
        NSLog(@"[liner] setActive failed: %@", error);
        liner_boot_log("audio_session: setActive FAILED");
        return 2;
    }

    liner_boot_log("audio_session: active");

    if (!gSessionObserversInstalled) {
        gSessionObserversInstalled = YES;
        NSNotificationCenter *center = [NSNotificationCenter defaultCenter];

        // A phone call / Siri / another app taking the session: pause, and
        // resume only when the system says we should.
        [center addObserverForName:AVAudioSessionInterruptionNotification
                            object:session
                             queue:[NSOperationQueue mainQueue]
                        usingBlock:^(NSNotification *note) {
            NSNumber *type = note.userInfo[AVAudioSessionInterruptionTypeKey];
            if (type.unsignedIntegerValue == AVAudioSessionInterruptionTypeBegan) {
                if (gRemoteCallback) gRemoteCallback(1); // pause
                return;
            }
            NSNumber *options = note.userInfo[AVAudioSessionInterruptionOptionKey];
            if (options.unsignedIntegerValue & AVAudioSessionInterruptionOptionShouldResume) {
                if (gRemoteCallback) gRemoteCallback(0); // play
            }
        }];

        // Headphones unplugged: pause rather than blast the speaker.
        [center addObserverForName:AVAudioSessionRouteChangeNotification
                            object:session
                             queue:[NSOperationQueue mainQueue]
                        usingBlock:^(NSNotification *note) {
            NSNumber *reason = note.userInfo[AVAudioSessionRouteChangeReasonKey];
            if (reason.unsignedIntegerValue == AVAudioSessionRouteChangeReasonOldDeviceUnavailable) {
                if (gRemoteCallback) gRemoteCallback(1);
            }
        }];

        // Media services reset (rare, usually after a crash in the audio server).
        [center addObserverForName:AVAudioSessionMediaServicesWereResetNotification
                            object:session
                             queue:[NSOperationQueue mainQueue]
                        usingBlock:^(NSNotification *note) {
            (void)note;
            [session setActive:YES withOptions:0 error:nil];
            if (gRemoteCallback) gRemoteCallback(0);
        }];
    }

    return 0;
}

void liner_audio_session_deactivate(void) {
    [[AVAudioSession sharedInstance] setActive:NO
                                   withOptions:AVAudioSessionSetActiveOptionNotifyOthersOnDeactivation
                                         error:nil];
}

#pragma mark - Now playing

void liner_now_playing_update(const char *title,
                              const char *artist,
                              const char *album,
                              double duration_s,
                              double elapsed_s,
                              double rate,
                              const char *artwork_path) {
    NSMutableDictionary<NSString *, id> *info = [NSMutableDictionary dictionary];

    NSString *titleString = LinerString(title);
    NSString *artistString = LinerString(artist);
    NSString *albumString = LinerString(album);

    if (titleString.length) info[MPMediaItemPropertyTitle] = titleString;
    if (artistString.length) info[MPMediaItemPropertyArtist] = artistString;
    if (albumString.length) info[MPMediaItemPropertyAlbumTitle] = albumString;

    info[MPMediaItemPropertyPlaybackDuration] = @(MAX(duration_s, 0.0));
    info[MPNowPlayingInfoPropertyElapsedPlaybackTime] = @(MAX(elapsed_s, 0.0));
    info[MPNowPlayingInfoPropertyPlaybackRate] = @(MAX(rate, 0.0));
    info[MPNowPlayingInfoPropertyMediaType] = @(MPNowPlayingInfoMediaTypeAudio);

    NSString *artworkFile = LinerString(artwork_path);
    if (artworkFile.length) {
        UIImage *image = [UIImage imageWithContentsOfFile:artworkFile];
        if (image != nil) {
            info[MPMediaItemPropertyArtwork] =
                [[MPMediaItemArtwork alloc] initWithBoundsSize:image.size
                                                requestHandler:^UIImage *(CGSize size) {
                (void)size;
                return image;
            }];
        }
    }

    MPNowPlayingInfoCenter *center = [MPNowPlayingInfoCenter defaultCenter];
    if (@available(iOS 13.0, *)) {
        center.playbackState = rate > 0.0 ? MPNowPlayingPlaybackStatePlaying
                                          : MPNowPlayingPlaybackStatePaused;
    }
    center.nowPlayingInfo = info;
}

/// Elapsed-time-only update: keeps the lock-screen scrubber in sync without
/// re-decoding artwork on every tick (5×/second).
void liner_now_playing_elapsed(double elapsed_s, double rate) {
    MPNowPlayingInfoCenter *center = [MPNowPlayingInfoCenter defaultCenter];
    NSMutableDictionary *info = [center.nowPlayingInfo mutableCopy];
    if (info == nil) return;
    info[MPNowPlayingInfoPropertyElapsedPlaybackTime] = @(MAX(elapsed_s, 0.0));
    info[MPNowPlayingInfoPropertyPlaybackRate] = @(MAX(rate, 0.0));
    center.nowPlayingInfo = info;
}

void liner_now_playing_rate(double rate) {
    MPNowPlayingInfoCenter *center = [MPNowPlayingInfoCenter defaultCenter];
    NSMutableDictionary *info = [center.nowPlayingInfo mutableCopy];
    if (info == nil) return;
    info[MPNowPlayingInfoPropertyPlaybackRate] = @(MAX(rate, 0.0));
    center.nowPlayingInfo = info;
}

void liner_now_playing_playing(int playing) {
    MPNowPlayingInfoCenter *center = [MPNowPlayingInfoCenter defaultCenter];
    if (@available(iOS 13.0, *)) {
        center.playbackState = playing ? MPNowPlayingPlaybackStatePlaying
                                       : MPNowPlayingPlaybackStatePaused;
    }
    NSMutableDictionary *info = [center.nowPlayingInfo mutableCopy];
    if (info == nil) return;
    info[MPNowPlayingInfoPropertyPlaybackRate] = @(playing ? 1.0 : 0.0);
    center.nowPlayingInfo = info;
}

void liner_now_playing_clear(void) {
    MPNowPlayingInfoCenter *center = [MPNowPlayingInfoCenter defaultCenter];
    center.nowPlayingInfo = nil;
    if (@available(iOS 13.0, *)) {
        center.playbackState = MPNowPlayingPlaybackStateStopped;
    }
}

#pragma mark - Remote commands

// Indices must match `platform_ios::REMOTE_*` in Rust.
enum {
    LinerRemotePlay = 0,
    LinerRemotePause = 1,
    LinerRemoteToggle = 2,
    LinerRemoteNext = 3,
    LinerRemotePrevious = 4,
    LinerRemoteSkipForward = 5,
    LinerRemoteSkipBackward = 6,
};

void liner_remote_commands_install(void (*callback)(int)) {
    gRemoteCallback = callback;

    MPRemoteCommandCenter *center = [MPRemoteCommandCenter sharedCommandCenter];

    [center.playCommand addTargetWithHandler:^MPRemoteCommandHandlerStatus(MPRemoteCommandEvent *event) {
        (void)event;
        if (gRemoteCallback) gRemoteCallback(LinerRemotePlay);
        return MPRemoteCommandHandlerStatusSuccess;
    }];

    [center.pauseCommand addTargetWithHandler:^MPRemoteCommandHandlerStatus(MPRemoteCommandEvent *event) {
        (void)event;
        if (gRemoteCallback) gRemoteCallback(LinerRemotePause);
        return MPRemoteCommandHandlerStatusSuccess;
    }];

    [center.togglePlayPauseCommand addTargetWithHandler:^MPRemoteCommandHandlerStatus(MPRemoteCommandEvent *event) {
        (void)event;
        if (gRemoteCallback) gRemoteCallback(LinerRemoteToggle);
        return MPRemoteCommandHandlerStatusSuccess;
    }];

    [center.nextTrackCommand addTargetWithHandler:^MPRemoteCommandHandlerStatus(MPRemoteCommandEvent *event) {
        (void)event;
        if (gRemoteCallback) gRemoteCallback(LinerRemoteNext);
        return MPRemoteCommandHandlerStatusSuccess;
    }];

    [center.previousTrackCommand addTargetWithHandler:^MPRemoteCommandHandlerStatus(MPRemoteCommandEvent *event) {
        (void)event;
        if (gRemoteCallback) gRemoteCallback(LinerRemotePrevious);
        return MPRemoteCommandHandlerStatusSuccess;
    }];

    center.skipForwardCommand.preferredIntervals = @[@15];
    [center.skipForwardCommand addTargetWithHandler:^MPRemoteCommandHandlerStatus(MPRemoteCommandEvent *event) {
        (void)event;
        if (gRemoteCallback) gRemoteCallback(LinerRemoteSkipForward);
        return MPRemoteCommandHandlerStatusSuccess;
    }];

    center.skipBackwardCommand.preferredIntervals = @[@15];
    [center.skipBackwardCommand addTargetWithHandler:^MPRemoteCommandHandlerStatus(MPRemoteCommandEvent *event) {
        (void)event;
        if (gRemoteCallback) gRemoteCallback(LinerRemoteSkipBackward);
        return MPRemoteCommandHandlerStatusSuccess;
    }];

    // Scrubbing from the lock screen: a seek has a position, so it travels
    // through its own latch rather than the integer command channel.
    [center.changePlaybackPositionCommand
        addTargetWithHandler:^MPRemoteCommandHandlerStatus(MPRemoteCommandEvent *event) {
        MPChangePlaybackPositionCommandEvent *seek = (MPChangePlaybackPositionCommandEvent *)event;
        gPendingSeek = seek.positionTime;
        return MPRemoteCommandHandlerStatusSuccess;
    }];

    center.playCommand.enabled = YES;
    center.pauseCommand.enabled = YES;
    center.togglePlayPauseCommand.enabled = YES;
    center.nextTrackCommand.enabled = YES;
    center.previousTrackCommand.enabled = YES;
    center.skipForwardCommand.enabled = YES;
    center.skipBackwardCommand.enabled = YES;
    center.changePlaybackPositionCommand.enabled = YES;
}

/// Returns the pending lock-screen seek in seconds, or a negative value when
/// there is none. Latched so a seek cannot be lost between audio ticks.
double liner_take_remote_seek(void) {
    double value = gPendingSeek;
    gPendingSeek = -1.0;
    return value;
}

#pragma mark - Document picker

static id gPickerDelegate = nil;

@interface LinerDocumentPicker : NSObject <UIDocumentPickerDelegate>
@property(nonatomic, copy) void (^completion)(NSArray<NSString *> *paths);
@end

@implementation LinerDocumentPicker

- (void)documentPicker:(UIDocumentPickerViewController *)controller
    didPickDocumentsAtURLs:(NSArray<NSURL *> *)urls {
    (void)controller;
    NSMutableArray<NSString *> *paths = [NSMutableArray arrayWithCapacity:urls.count];
    for (NSURL *url in urls) {
        // `asCopy:YES` already materialised the file inside our container.
        if (url.path.length) [paths addObject:url.path];
    }
    if (self.completion) self.completion(paths);
    gPickerDelegate = nil;
}

- (void)documentPickerWasCancelled:(UIDocumentPickerViewController *)controller {
    (void)controller;
    if (self.completion) self.completion(@[]);
    gPickerDelegate = nil;
}

@end

static UIViewController *LinerTopViewController(void) {
    UIWindow *window = nil;
    for (UIScene *scene in [UIApplication sharedApplication].connectedScenes) {
        if (![scene isKindOfClass:[UIWindowScene class]]) continue;
        UIWindowScene *windowScene = (UIWindowScene *)scene;
        for (UIWindow *candidate in windowScene.windows) {
            if (candidate.isKeyWindow) {
                window = candidate;
                break;
            }
        }
        if (window != nil) break;
    }

    UIViewController *controller = window.rootViewController;
    while (controller.presentedViewController != nil) {
        controller = controller.presentedViewController;
    }
    return controller;
}

void liner_pick_audio_files(void (*callback)(const char *json_paths)) {
    dispatch_async(dispatch_get_main_queue(), ^{
        UIViewController *host = LinerTopViewController();
        if (host == nil) {
            callback("[]");
            return;
        }

        LinerDocumentPicker *delegate = [LinerDocumentPicker new];
        delegate.completion = ^(NSArray<NSString *> *paths) {
            NSData *data = [NSJSONSerialization dataWithJSONObject:paths ?: @[] options:0 error:nil];
            NSString *json = data ? [[NSString alloc] initWithData:data encoding:NSUTF8StringEncoding]
                                  : @"[]";
            callback(json.UTF8String);
        };
        gPickerDelegate = delegate;

        UIDocumentPickerViewController *picker =
            [[UIDocumentPickerViewController alloc] initForOpeningContentTypes:@[ UTTypeAudio ]
                                                                       asCopy:YES];
        picker.delegate = delegate;
        picker.allowsMultipleSelection = YES;
        picker.shouldShowFileExtensions = YES;

        [host presentViewController:picker animated:YES completion:nil];
    });
}

#pragma mark - Misc

int liner_open_url(const char *url) {
    NSString *string = LinerString(url);
    if (string.length == 0) return 0;
    NSURL *parsed = [NSURL URLWithString:string];
    if (parsed == nil) return 0;

    dispatch_async(dispatch_get_main_queue(), ^{
        [[UIApplication sharedApplication] openURL:parsed options:@{} completionHandler:nil];
    });
    return 1;
}

/// While music plays the screen should not dim — the lock screen is the
/// primary surface for a player.
void liner_idle_timer_disabled(int disabled) {
    dispatch_async(dispatch_get_main_queue(), ^{
        [UIApplication sharedApplication].idleTimerDisabled = (disabled != 0);
    });
}

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
