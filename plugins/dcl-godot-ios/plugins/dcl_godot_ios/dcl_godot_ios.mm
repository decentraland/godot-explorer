#include "dcl_godot_ios.h"
#include "core/version.h"
#import <SafariServices/SafariServices.h>
#import <AuthenticationServices/AuthenticationServices.h>

const char* DCLGODOTIOS_VERSION = "1.0";

// Custom view controller to enforce portrait orientation
@interface PortraitViewController : UIViewController
@end

@implementation PortraitViewController
- (UIInterfaceOrientationMask)supportedInterfaceOrientations {
    return UIInterfaceOrientationMaskPortrait;
}

- (UIInterfaceOrientation)preferredInterfaceOrientationForPresentation {
    return UIInterfaceOrientationPortrait;
}
@end

// Helper class to act as the presentation context provider for ASWebAuthenticationSession
@interface WebKitAuthenticationDelegate : NSObject <ASWebAuthenticationPresentationContextProviding>
@property (nonatomic, strong) UIWindow *authWindow;

- (void)show_notification_in_auth_window:(NSString *)message;

@end

@implementation WebKitAuthenticationDelegate

- (UIWindow *)presentationAnchorForWebAuthenticationSession:(ASWebAuthenticationSession *)session {
    if (!self.authWindow) {
        self.authWindow = [[UIWindow alloc] initWithFrame:[[UIScreen mainScreen] bounds]];
        self.authWindow.rootViewController = [[PortraitViewController alloc] init];
        self.authWindow.windowLevel = UIWindowLevelAlert + 1;
        [self.authWindow makeKeyAndVisible];
        
        // Force the orientation to portrait using UIWindowScene API if available on iOS 16 or later
        [self setWindowOrientation:UIInterfaceOrientationPortrait];
    }
    return self.authWindow;
}

- (void)setWindowOrientation:(UIInterfaceOrientation)orientation {
    UIWindowScene *windowScene = (UIWindowScene *)self.authWindow.windowScene;
    if (windowScene) {
        UIInterfaceOrientationMask orientationMask = (orientation == UIInterfaceOrientationPortrait) ? UIInterfaceOrientationMaskPortrait : UIInterfaceOrientationMaskLandscape;
        
        UIWindowSceneGeometryPreferencesIOS *geometryPreferences = [[UIWindowSceneGeometryPreferencesIOS alloc] init];
        geometryPreferences.interfaceOrientations = orientationMask;
        
        [windowScene requestGeometryUpdateWithPreferences:geometryPreferences errorHandler:^(NSError *error) {
            NSLog(@"Error setting window orientation: %@", error.localizedDescription);
        }];
    }
}

- (void)show_notification_in_auth_window:(NSString *)message {
    if (!self.authWindow || message.length == 0) return;

    UILabel *toastLabel = [[UILabel alloc] initWithFrame:CGRectMake(0, 0, self.authWindow.frame.size.width - 40, 50)];
    toastLabel.backgroundColor = [[UIColor blackColor] colorWithAlphaComponent:0.7];
    toastLabel.textColor = [UIColor whiteColor];
    toastLabel.textAlignment = NSTextAlignmentCenter;
    toastLabel.text = message;
    toastLabel.alpha = 0.0;
    toastLabel.layer.cornerRadius = 10;
    toastLabel.clipsToBounds = YES;

    // Position the label at the bottom of the auth window
    toastLabel.center = CGPointMake(self.authWindow.center.x, self.authWindow.frame.size.height - 100);

    [self.authWindow addSubview:toastLabel];

    // Animate the appearance and disappearance of the toast
    [UIView animateWithDuration:0.5
                          delay:0
                        options:UIViewAnimationOptionCurveEaseInOut
                     animations:^{
                         toastLabel.alpha = 1.0;
                     }
                     completion:^(BOOL finished) {
                         [UIView animateWithDuration:0.5
                                               delay:2.0
                                             options:UIViewAnimationOptionCurveEaseInOut
                                          animations:^{
                                              toastLabel.alpha = 0.0;
                                          }
                                          completion:^(BOOL finished) {
                                              [toastLabel removeFromSuperview];
                                          }];
                     }];
}

- (void)dealloc {
    self.authWindow.hidden = YES;
    self.authWindow = nil;
}

@end

DclGodotiOS *DclGodotiOS::instance = NULL;

void DclGodotiOS::_bind_methods() {
    ClassDB::bind_method(D_METHOD("print_version"), &DclGodotiOS::print_version);
    ClassDB::bind_method(D_METHOD("open_auth_url", "url"), &DclGodotiOS::open_auth_url);
}

void DclGodotiOS::print_version() {
    printf("DclGodotiOS Version %s - Godot: %s\n", DCLGODOTIOS_VERSION, VERSION_FULL_NAME);
}

void DclGodotiOS::open_auth_url(String url) {
    #if TARGET_OS_IOS
    NSString *ns_url = [NSString stringWithUTF8String:url.utf8().get_data()];
    NSURL *ns_nsurl = [NSURL URLWithString:ns_url];
    NSString *callbackScheme = @"decentraland";

    // Initialize the helper delegate with portrait enforcement
    authDelegate = [[WebKitAuthenticationDelegate alloc] init];

    // Retain the session instance in the class
    authSession = [[ASWebAuthenticationSession alloc]
        initWithURL:ns_nsurl
        callbackURLScheme:callbackScheme
        completionHandler:^(NSURL * _Nullable callbackURL, NSError * _Nullable error) {
            if (callbackURL) {
                printf("Authentication completed with callback URL: %s\n", callbackURL.absoluteString.UTF8String);
                // Forward the callback URL to Godot if needed
            } else if (error) {
                printf("Authentication failed with error: %s\n", error.localizedDescription.UTF8String);
                // Forward the error to Godot if needed
            }

            // Release the authSession and remove the auth window
            authSession = nil;
            authDelegate.authWindow.hidden = YES;
            authDelegate.authWindow = nil;
        }];

    authSession.presentationContextProvider = authDelegate;
    [authSession start];
    #endif
}

void DclGodotiOS::open_webview_url(String url) {
    #if TARGET_OS_IOS
    NSString *ns_url = [NSString stringWithUTF8String:url.utf8().get_data()];
    NSURL *ns_nsurl = [NSURL URLWithString:ns_url];

    dispatch_async(dispatch_get_main_queue(), ^{
        // Create Safari View Controller
        SFSafariViewController *safariVC = [[SFSafariViewController alloc] initWithURL:ns_nsurl];
        safariVC.modalPresentationStyle = UIModalPresentationPageSheet;

        // Get the top-most view controller
        UIViewController *rootVC = [UIApplication sharedApplication].keyWindow.rootViewController;
        while (rootVC.presentedViewController) {
            rootVC = rootVC.presentedViewController;
        }

        [rootVC presentViewController:safariVC animated:YES completion:nil];
    });
    #endif
}

DclGodotiOS *DclGodotiOS::get_singleton() {
    return instance;
}

DclGodotiOS::DclGodotiOS() {
    instance = this;
    authSession = nullptr;
    authDelegate = nullptr;
}

DclGodotiOS::~DclGodotiOS() {
    instance = NULL;
    authSession = nullptr;
    authDelegate = nullptr;
}
