// Liner Music — Siri / Shortcuts intents extension.
//
// `INPlayMediaIntent` is ObjC (Intents.framework), so this compiles where App
// Intents (Swift) cannot. It answers Siri's media requests by handing off to the
// app, which then does the actual playback through the web player.
//
// Kept deliberately minimal: the SDK's Intents surface varies between versions
// (INPauseMediaIntentResponse, for example, is absent here), and a single
// well-formed handler is what makes the extension load and appear in Shortcuts.

#import <Intents/Intents.h>

@interface IntentHandler : INExtension <INPlayMediaIntentHandling>
@end

@implementation IntentHandler

- (id)handlerForIntent:(INIntent *)intent {
    if ([intent isKindOfClass:[INPlayMediaIntent class]]) {
        return self;
    }
    return nil;
}

- (void)handlePlayMedia:(INPlayMediaIntent *)intent
             completion:(void (^)(INPlayMediaIntentResponse *response))completion {
    (void)intent;

    NSUserActivity *activity =
        [[NSUserActivity alloc] initWithActivityType:@"com.iagsoftware.linermusic.play"];

    completion([[INPlayMediaIntentResponse alloc]
        initWithCode:INPlayMediaIntentResponseCodeContinueInApp
        userActivity:activity]);
}

@end
