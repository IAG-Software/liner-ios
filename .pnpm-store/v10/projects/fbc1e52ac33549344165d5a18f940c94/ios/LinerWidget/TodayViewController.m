// Liner Music — Today widget (Notification Center extension).
//
// WHY THIS SHAPE: home-screen widgets are WidgetKit, which is SwiftUI-only and
// therefore unreachable from this toolchain. The older *Today* extension
// (`com.apple.widget-extension`) is plain ObjC + UIKit and still loads on modern
// iOS in the Today view, so that is what this is — a real widget bundle rather
// than a fake.
//
// It shows a Liner card with a button that deep-links back into the app through
// the `liner://` scheme.

#import <NotificationCenter/NotificationCenter.h>
#import <UIKit/UIKit.h>

@interface TodayViewController : UIViewController <NCWidgetProviding>
@property(nonatomic, strong) UILabel *titleLabel;
@property(nonatomic, strong) UILabel *subtitleLabel;
@end

@implementation TodayViewController

- (void)viewDidLoad {
    [super viewDidLoad];

    self.view.backgroundColor = [UIColor colorWithWhite:0.08 alpha:1.0];

    UILabel *wordmark = [[UILabel alloc] initWithFrame:CGRectZero];
    wordmark.text = @"Liner";
    wordmark.font = [UIFont systemFontOfSize:17 weight:UIFontWeightSemibold];
    wordmark.textColor = [UIColor whiteColor];
    wordmark.translatesAutoresizingMaskIntoConstraints = NO;
    [self.view addSubview:wordmark];

    self.subtitleLabel = [[UILabel alloc] initWithFrame:CGRectZero];
    self.subtitleLabel.text = @"Open Liner to see what's playing";
    self.subtitleLabel.font = [UIFont systemFontOfSize:12 weight:UIFontWeightRegular];
    self.subtitleLabel.textColor = [UIColor colorWithWhite:1 alpha:0.55];
    self.subtitleLabel.numberOfLines = 2;
    self.subtitleLabel.translatesAutoresizingMaskIntoConstraints = NO;
    [self.view addSubview:self.subtitleLabel];

    UIButton *open = [UIButton buttonWithType:UIButtonTypeSystem];
    [open setTitle:@"Open Liner" forState:UIControlStateNormal];
    open.titleLabel.font = [UIFont systemFontOfSize:13 weight:UIFontWeightMedium];
    open.tintColor = [UIColor whiteColor];
    open.backgroundColor = [UIColor colorWithWhite:1 alpha:0.14];
    open.layer.cornerRadius = 10;
    open.layer.cornerCurve = kCACornerCurveContinuous;
    open.translatesAutoresizingMaskIntoConstraints = NO;
    [open addTarget:self action:@selector(openApp) forControlEvents:UIControlEventTouchUpInside];
    [self.view addSubview:open];

    UILayoutGuide *safe = self.view.safeAreaLayoutGuide;
    [NSLayoutConstraint activateConstraints:@[
        [wordmark.leadingAnchor constraintEqualToAnchor:safe.leadingAnchor constant:16],
        [wordmark.topAnchor constraintEqualToAnchor:safe.topAnchor constant:10],

        [self.subtitleLabel.leadingAnchor constraintEqualToAnchor:wordmark.leadingAnchor],
        [self.subtitleLabel.trailingAnchor constraintEqualToAnchor:open.leadingAnchor constant:-12],
        [self.subtitleLabel.topAnchor constraintEqualToAnchor:wordmark.bottomAnchor constant:2],
        [self.subtitleLabel.bottomAnchor constraintLessThanOrEqualToAnchor:safe.bottomAnchor constant:-8],

        [open.trailingAnchor constraintEqualToAnchor:safe.trailingAnchor constant:-16],
        [open.centerYAnchor constraintEqualToAnchor:wordmark.centerYAnchor constant:6],
        [open.widthAnchor constraintEqualToConstant:104],
        [open.heightAnchor constraintEqualToConstant:34],
    ]];
}

/// Extensions cannot launch their host arbitrarily; `extensionContext` is the
/// supported route and iOS turns it into an `openURL` on the app.
- (void)openApp {
    NSURL *url = [NSURL URLWithString:@"liner://library"];
    if (url == nil) return;
    [self.extensionContext openURL:url completionHandler:nil];
}

- (void)widgetPerformUpdateWithCompletionHandler:
    (void (^)(NCUpdateResult))completionHandler {
    completionHandler(NCUpdateResultNewData);
}

- (NCWidgetDisplayMode)widgetActiveDisplayMode {
    return NCWidgetDisplayModeCompact;
}

- (CGSize)preferredContentSize {
    return CGSizeMake(0, 84);
}

@end
