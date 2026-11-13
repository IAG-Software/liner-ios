// Liner Music — iMessage app extension (stub that actually loads).
//
// `Messages` is an ObjC framework, so unlike WidgetKit/SwiftUI this one can be
// compiled here. The extension lets you drop a Liner card into a conversation;
// tapping it opens the app through the `liner://` scheme.
//
// Packaging caveat: this ships as a nested `PlugIns/MessagesExtension.appex`.
// Sideload re-signers vary in how they handle nested bundles, so it may install
// without the iMessage app appearing. The bundle is structurally correct either
// way.

#import <Messages/Messages.h>
#import <UIKit/UIKit.h>

@interface MessagesViewController : MSMessagesAppViewController
@end

@implementation MessagesViewController

- (void)viewDidLoad {
    [super viewDidLoad];

    self.view.backgroundColor = [UIColor colorWithWhite:0.08 alpha:1.0];

    UILabel *label = [[UILabel alloc] initWithFrame:CGRectZero];
    label.text = @"Share Liner\nSend a card that opens the app.";
    label.numberOfLines = 0;
    label.textAlignment = NSTextAlignmentCenter;
    label.font = [UIFont systemFontOfSize:16 weight:UIFontWeightSemibold];
    label.textColor = [UIColor whiteColor];
    label.translatesAutoresizingMaskIntoConstraints = NO;
    [self.view addSubview:label];

    UIButton *send = [UIButton buttonWithType:UIButtonTypeSystem];
    [send setTitle:@"Send Liner card" forState:UIControlStateNormal];
    send.titleLabel.font = [UIFont systemFontOfSize:14 weight:UIFontWeightMedium];
    send.tintColor = [UIColor whiteColor];
    send.backgroundColor = [UIColor colorWithWhite:1 alpha:0.16];
    send.layer.cornerRadius = 12;
    send.layer.cornerCurve = kCACornerCurveContinuous;
    send.translatesAutoresizingMaskIntoConstraints = NO;
    [send addTarget:self action:@selector(sendCard) forControlEvents:UIControlEventTouchUpInside];
    [self.view addSubview:send];

    UILayoutGuide *safe = self.view.safeAreaLayoutGuide;
    [NSLayoutConstraint activateConstraints:@[
        [label.leadingAnchor constraintEqualToAnchor:safe.leadingAnchor constant:20],
        [label.trailingAnchor constraintEqualToAnchor:safe.trailingAnchor constant:-20],
        [label.centerYAnchor constraintEqualToAnchor:safe.centerYAnchor constant:-24],

        [send.centerXAnchor constraintEqualToAnchor:safe.centerXAnchor],
        [send.topAnchor constraintEqualToAnchor:label.bottomAnchor constant:18],
        [send.widthAnchor constraintEqualToConstant:180],
        [send.heightAnchor constraintEqualToConstant:40],
    ]];
}

- (void)sendCard {
    if (self.activeConversation == nil) return;

    NSURL *url = [NSURL URLWithString:@"liner://library"];
    MSMessageTemplateLayout *layout = [[MSMessageTemplateLayout alloc] init];
    layout.caption = @"Listening on Liner";
    layout.subcaption = @"Tap to open";

    MSMessage *message = [[MSMessage alloc] init];
    message.URL = url;
    message.layout = layout;

    [self.activeConversation insertMessage:message completionHandler:nil];
}

@end
