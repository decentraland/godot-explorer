#include "dcl_godot_ios.h"
#include "NotificationDatabase.h"
#include "AVPlayerWrapper.h"
#include "core/version.h"
#import <SafariServices/SafariServices.h>
#import <AuthenticationServices/AuthenticationServices.h>
#import <UIKit/UIKit.h>
#import <Foundation/Foundation.h>
#import <Network/Network.h>
#import <sys/sysctl.h>
#import <mach/mach.h>
#import <EventKit/EventKit.h>
#import <EventKitUI/EventKitUI.h>
#import <LinkPresentation/LinkPresentation.h>
#import <UserNotifications/UserNotifications.h>

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

// Helper class to handle calendar event edit view controller delegate
@interface CalendarEventDelegate : NSObject <EKEventEditViewDelegate>
@end

@implementation CalendarEventDelegate

- (void)eventEditViewController:(EKEventEditViewController *)controller didCompleteWithAction:(EKEventEditViewAction)action {
    // Dismiss the event edit view controller
    [controller.presentingViewController dismissViewControllerAnimated:YES completion:^{
        if (action == EKEventEditViewActionSaved) {
            printf("Calendar event saved successfully\n");
        } else if (action == EKEventEditViewActionCanceled) {
            printf("Calendar event creation cancelled\n");
        } else if (action == EKEventEditViewActionDeleted) {
            printf("Calendar event deleted\n");
        }
    }];
}

@end

DclGodotiOS *DclGodotiOS::instance = NULL;
String DclGodotiOS::receivedUrl = "";

void DclGodotiOS::_bind_methods() {
    ClassDB::bind_method(D_METHOD("print_version"), &DclGodotiOS::print_version);
    ClassDB::bind_method(D_METHOD("open_auth_url", "url"), &DclGodotiOS::open_auth_url);
    ClassDB::bind_method(D_METHOD("open_webview_url", "url"), &DclGodotiOS::open_webview_url);
    ClassDB::bind_method(D_METHOD("get_deeplink_url"), &DclGodotiOS::get_deeplink_url);
    ClassDB::bind_method(D_METHOD("get_mobile_device_info"), &DclGodotiOS::get_mobile_device_info);
    ClassDB::bind_method(D_METHOD("get_mobile_metrics"), &DclGodotiOS::get_mobile_metrics);
    ClassDB::bind_method(D_METHOD("add_calendar_event", "title", "description", "start_time", "end_time", "location"), &DclGodotiOS::add_calendar_event);
    ClassDB::bind_method(D_METHOD("share_text", "text"), &DclGodotiOS::share_text);
    ClassDB::bind_method(D_METHOD("share_text_with_image", "text", "image"), &DclGodotiOS::share_text_with_image);

    // Local notifications - Phase 1
    ClassDB::bind_method(D_METHOD("request_notification_permission"), &DclGodotiOS::request_notification_permission);
    ClassDB::bind_method(D_METHOD("has_notification_permission"), &DclGodotiOS::has_notification_permission);
    ClassDB::bind_method(D_METHOD("schedule_local_notification", "notification_id", "title", "body", "delay_seconds"), &DclGodotiOS::schedule_local_notification);
    ClassDB::bind_method(D_METHOD("cancel_local_notification", "notification_id"), &DclGodotiOS::cancel_local_notification);
    ClassDB::bind_method(D_METHOD("cancel_all_local_notifications"), &DclGodotiOS::cancel_all_local_notifications);
    ClassDB::bind_method(D_METHOD("clear_badge_number"), &DclGodotiOS::clear_badge_number);

    // Database API - Phase 3
    ClassDB::bind_method(D_METHOD("db_insert_notification", "id", "title", "body", "trigger_timestamp", "is_scheduled", "data", "image_base64"), &DclGodotiOS::db_insert_notification);
    ClassDB::bind_method(D_METHOD("db_update_notification", "id", "updates"), &DclGodotiOS::db_update_notification);
    ClassDB::bind_method(D_METHOD("db_delete_notification", "id"), &DclGodotiOS::db_delete_notification);
    ClassDB::bind_method(D_METHOD("db_query_notifications", "where_clause", "order_by", "limit"), &DclGodotiOS::db_query_notifications);
    ClassDB::bind_method(D_METHOD("db_count_notifications", "where_clause"), &DclGodotiOS::db_count_notifications);
    ClassDB::bind_method(D_METHOD("db_clear_expired", "current_timestamp"), &DclGodotiOS::db_clear_expired);
    ClassDB::bind_method(D_METHOD("db_mark_scheduled", "id", "is_scheduled"), &DclGodotiOS::db_mark_scheduled);
    ClassDB::bind_method(D_METHOD("db_get_notification", "id"), &DclGodotiOS::db_get_notification);
    ClassDB::bind_method(D_METHOD("db_clear_all"), &DclGodotiOS::db_clear_all);
    ClassDB::bind_method(D_METHOD("db_get_notification_image_blob", "id"), &DclGodotiOS::db_get_notification_image_blob);

    // OS Notification API - Phase 3
    ClassDB::bind_method(D_METHOD("os_schedule_notification", "notification_id", "title", "body", "delay_seconds"), &DclGodotiOS::os_schedule_notification);
    ClassDB::bind_method(D_METHOD("os_cancel_notification", "notification_id"), &DclGodotiOS::os_cancel_notification);
    ClassDB::bind_method(D_METHOD("os_get_scheduled_ids"), &DclGodotiOS::os_get_scheduled_ids);

    // AVPlayer API
    ClassDB::bind_method(D_METHOD("createAVPlayer"), &DclGodotiOS::createAVPlayer);
    ClassDB::bind_method(D_METHOD("avPlayerRelease", "player_id"), &DclGodotiOS::avPlayerRelease);
    ClassDB::bind_method(D_METHOD("avPlayerInitSurface", "player_id", "width", "height"), &DclGodotiOS::avPlayerInitSurface);
    ClassDB::bind_method(D_METHOD("avPlayerSetSourceUrl", "player_id", "url"), &DclGodotiOS::avPlayerSetSourceUrl);
    ClassDB::bind_method(D_METHOD("avPlayerSetSourceLocal", "player_id", "file_path"), &DclGodotiOS::avPlayerSetSourceLocal);
    ClassDB::bind_method(D_METHOD("avPlayerPlay", "player_id"), &DclGodotiOS::avPlayerPlay);
    ClassDB::bind_method(D_METHOD("avPlayerPause", "player_id"), &DclGodotiOS::avPlayerPause);
    ClassDB::bind_method(D_METHOD("avPlayerStop", "player_id"), &DclGodotiOS::avPlayerStop);
    ClassDB::bind_method(D_METHOD("avPlayerSetPosition", "player_id", "position_sec"), &DclGodotiOS::avPlayerSetPosition);
    ClassDB::bind_method(D_METHOD("avPlayerGetPosition", "player_id"), &DclGodotiOS::avPlayerGetPosition);
    ClassDB::bind_method(D_METHOD("avPlayerGetDuration", "player_id"), &DclGodotiOS::avPlayerGetDuration);
    ClassDB::bind_method(D_METHOD("avPlayerIsPlaying", "player_id"), &DclGodotiOS::avPlayerIsPlaying);
    ClassDB::bind_method(D_METHOD("avPlayerGetVideoWidth", "player_id"), &DclGodotiOS::avPlayerGetVideoWidth);
    ClassDB::bind_method(D_METHOD("avPlayerGetVideoHeight", "player_id"), &DclGodotiOS::avPlayerGetVideoHeight);
    ClassDB::bind_method(D_METHOD("avPlayerHasVideoSizeChanged", "player_id"), &DclGodotiOS::avPlayerHasVideoSizeChanged);
    ClassDB::bind_method(D_METHOD("avPlayerGetTextureWidth", "player_id"), &DclGodotiOS::avPlayerGetTextureWidth);
    ClassDB::bind_method(D_METHOD("avPlayerGetTextureHeight", "player_id"), &DclGodotiOS::avPlayerGetTextureHeight);
    ClassDB::bind_method(D_METHOD("avPlayerSetVolume", "player_id", "volume"), &DclGodotiOS::avPlayerSetVolume);
    ClassDB::bind_method(D_METHOD("avPlayerGetVolume", "player_id"), &DclGodotiOS::avPlayerGetVolume);
    ClassDB::bind_method(D_METHOD("avPlayerSetLooping", "player_id", "loop"), &DclGodotiOS::avPlayerSetLooping);
    ClassDB::bind_method(D_METHOD("avPlayerSetPlaybackRate", "player_id", "rate"), &DclGodotiOS::avPlayerSetPlaybackRate);
    ClassDB::bind_method(D_METHOD("avPlayerHasNewPixelBuffer", "player_id"), &DclGodotiOS::avPlayerHasNewPixelBuffer);
    ClassDB::bind_method(D_METHOD("avPlayerAcquireIOSurfacePtr", "player_id"), &DclGodotiOS::avPlayerAcquireIOSurfacePtr);
    ClassDB::bind_method(D_METHOD("avPlayerGetInfo", "player_id"), &DclGodotiOS::avPlayerGetInfo);

    // Signal emitted when a deeplink URL is received
    ADD_SIGNAL(MethodInfo("deeplink_received", PropertyInfo(Variant::STRING, "url")));
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
                NSLog(@"[DEEPLINK] ASWebAuthenticationSession completed with callback URL: %@", callbackURL.absoluteString);
                // Only forward if the callback URL has the decentraland:// scheme
                if ([callbackURL.scheme isEqualToString:@"decentraland"]) {
                    DclGodotiOS::emit_deeplink_received(String::utf8(callbackURL.absoluteString.UTF8String));
                } else {
                    NSLog(@"[DEEPLINK] Ignoring callback URL with unexpected scheme: %@", callbackURL.scheme);
                }
            } else if (error) {
                NSLog(@"[DEEPLINK] ASWebAuthenticationSession failed with error: %@", error.localizedDescription);
            }

            // Release the authSession and remove the auth window
            authSession = nil;
            authDelegate.authWindow.hidden = YES;
            authDelegate.authWindow = nil;
        }];

    authSession.presentationContextProvider = authDelegate;

    // Use ephemeral session to ensure clean state (no shared cookies/session with Safari)
    // This ensures each auth attempt starts fresh without previous login sessions
    authSession.prefersEphemeralWebBrowserSession = YES;

    BOOL started = [authSession start];
    if (!started) {
        [authDelegate show_notification_in_auth_window:@"Failed to start auth session"];
    }
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

String DclGodotiOS::get_deeplink_url() {
    String url = receivedUrl;
    NSLog(@"[DEEPLINK] get_deeplink_url called, returning: %s", url.utf8().get_data());
    // Only clear the URL if we're actually returning something
    // This prevents race conditions where get_deeplink_url() is called
    // before openURL/continueUserActivity has been invoked
    if (!url.is_empty()) {
        receivedUrl = "";
    }
    return url;
}

void DclGodotiOS::emit_deeplink_received(String url) {
    NSLog(@"[DEEPLINK] emit_deeplink_received called with URL: %s", url.utf8().get_data());

    // Always store the URL in the static variable as a fallback
    // This ensures it's available even if the singleton isn't initialized yet
    receivedUrl = url;
    NSLog(@"[DEEPLINK] URL stored in static receivedUrl");

    // Try to emit signal if singleton is available
    DclGodotiOS *singleton = get_singleton();
    if (singleton) {
        singleton->emit_signal("deeplink_received", url);
        NSLog(@"[DEEPLINK] Signal emitted successfully");
    } else {
        NSLog(@"[DEEPLINK] WARNING: Singleton not available yet, URL stored in static variable only");
    }
}

Dictionary DclGodotiOS::get_mobile_device_info() {
    Dictionary info;

    #if TARGET_OS_IOS
    // Device brand
    info["device_brand"] = "Apple";

    // Get device model using sysctlbyname
    char model_buffer[256];
    size_t model_size = sizeof(model_buffer);
    if (sysctlbyname("hw.machine", model_buffer, &model_size, NULL, 0) == 0) {
        info["device_model"] = String(model_buffer);
    } else {
        // Fallback to UIDevice model
        NSString *model = [[UIDevice currentDevice] model];
        info["device_model"] = String(model.UTF8String);
    }

    // OS version
    NSString *os_version = [[UIDevice currentDevice] systemVersion];
    info["os_version"] = String([NSString stringWithFormat:@"iOS %@", os_version].UTF8String);

    // Get total RAM using sysctl
    uint64_t total_memory = 0;
    size_t size = sizeof(total_memory);
    if (sysctlbyname("hw.memsize", &total_memory, &size, NULL, 0) == 0) {
        info["total_ram_mb"] = (int)(total_memory / (1024 * 1024));
    } else {
        info["total_ram_mb"] = 0;
    }
    #endif

    return info;
}

Dictionary DclGodotiOS::get_mobile_metrics() {
    Dictionary metrics;

    #if TARGET_OS_IOS
    // Enable battery monitoring
    [[UIDevice currentDevice] setBatteryMonitoringEnabled:YES];

    // Get thermal state
    NSProcessInfo *processInfo = [NSProcessInfo processInfo];
    NSString *thermalState = @"unknown";
    switch (processInfo.thermalState) {
        case NSProcessInfoThermalStateNominal:
            thermalState = @"nominal";
            break;
        case NSProcessInfoThermalStateFair:
            thermalState = @"fair";
            break;
        case NSProcessInfoThermalStateSerious:
            thermalState = @"serious";
            break;
        case NSProcessInfoThermalStateCritical:
            thermalState = @"critical";
            break;
    }
    metrics["thermal_state"] = String(thermalState.UTF8String);

    // Get battery state and level
    UIDeviceBatteryState batteryState = [[UIDevice currentDevice] batteryState];
    float battery_percent = [[UIDevice currentDevice] batteryLevel] * 100.0f; // 0-100
    metrics["battery_percent"] = battery_percent;

    // Map battery state to charging state string
    NSString *chargingState;
    switch (batteryState) {
        case UIDeviceBatteryStateUnknown:
            chargingState = @"unknown";
            break;
        case UIDeviceBatteryStateUnplugged:
            chargingState = @"unplugged";
            break;
        case UIDeviceBatteryStateCharging:
            chargingState = @"plugged";  // iOS doesn't differentiate USB/wireless
            break;
        case UIDeviceBatteryStateFull:
            chargingState = @"full";
            break;
    }
    metrics["charging_state"] = String(chargingState.UTF8String);

    // Get temperature (battery temperature approximation)
    // iOS doesn't expose CPU/device temperature directly, so we use -1.0 as placeholder
    metrics["device_temperature_celsius"] = -1.0f;

    // Get RAM consumption using phys_footprint (what Xcode uses)
    struct task_vm_info vm_info;
    mach_msg_type_number_t vm_info_count = TASK_VM_INFO_COUNT;
    if (task_info(mach_task_self(), TASK_VM_INFO, (task_info_t)&vm_info, &vm_info_count) == KERN_SUCCESS) {
        // phys_footprint is the actual physical memory used (what Xcode shows)
        metrics["memory_usage"] = (int)(vm_info.phys_footprint / (1024 * 1024));
    } else {
        metrics["memory_usage"] = 0;
    }
    #endif

    return metrics;
}

bool DclGodotiOS::add_calendar_event(String title, String description, int64_t start_time, int64_t end_time, String location) {
    #if TARGET_OS_IOS
    dispatch_async(dispatch_get_main_queue(), ^{
        EKEventStore *eventStore = [[EKEventStore alloc] init];

        // Create the event
        // Note: EKEventEditViewController handles permissions internally,
        // so we don't need to request authorization beforehand
        EKEvent *event = [EKEvent eventWithEventStore:eventStore];
        event.title = [NSString stringWithUTF8String:title.utf8().get_data()];
        event.notes = [NSString stringWithUTF8String:description.utf8().get_data()];
        event.location = [NSString stringWithUTF8String:location.utf8().get_data()];

        // Convert timestamps (milliseconds) to NSDate
        event.startDate = [NSDate dateWithTimeIntervalSince1970:(start_time / 1000.0)];
        event.endDate = [NSDate dateWithTimeIntervalSince1970:(end_time / 1000.0)];
        event.calendar = [eventStore defaultCalendarForNewEvents];

        // Create and retain the delegate
        calendarDelegate = [[CalendarEventDelegate alloc] init];

        // Create the event edit view controller
        // This view controller will request calendar access if needed
        EKEventEditViewController *eventEditVC = [[EKEventEditViewController alloc] init];
        eventEditVC.event = event;
        eventEditVC.eventStore = eventStore;
        eventEditVC.editViewDelegate = calendarDelegate;

        // Get the top-most view controller
        UIViewController *rootVC = [UIApplication sharedApplication].keyWindow.rootViewController;
        while (rootVC.presentedViewController) {
            rootVC = rootVC.presentedViewController;
        }

        [rootVC presentViewController:eventEditVC animated:YES completion:^{
            printf("Calendar event editor presented successfully\n");
        }];
    });

    return true;
    #else
    return false;
    #endif
}

bool DclGodotiOS::share_text(String text) {
    #if TARGET_OS_IOS
    NSString *ns_text = [NSString stringWithUTF8String:text.utf8().get_data()];

    dispatch_async(dispatch_get_main_queue(), ^{
        NSMutableArray *activityItems = [NSMutableArray array];

        // Detect URLs in the text using NSDataDetector
        NSError *error = nil;
        NSDataDetector *detector = [NSDataDetector dataDetectorWithTypes:NSTextCheckingTypeLink error:&error];

        if (detector) {
            NSArray *matches = [detector matchesInString:ns_text options:0 range:NSMakeRange(0, [ns_text length])];

            if (matches.count > 0) {
                // Found at least one URL - add both text and URL for rich preview
                [activityItems addObject:ns_text];

                // Add the first URL found for rich link preview
                NSTextCheckingResult *firstMatch = matches[0];
                NSURL *url = firstMatch.URL;
                if (url) {
                    [activityItems addObject:url];
                    printf("Sharing with URL for rich preview: %s\n", url.absoluteString.UTF8String);
                }
            } else {
                // No URL found, just share the text
                [activityItems addObject:ns_text];
            }
        } else {
            // Error creating detector, fallback to plain text
            [activityItems addObject:ns_text];
            if (error) {
                printf("Error creating URL detector: %s\n", error.localizedDescription.UTF8String);
            }
        }

        // Create the activity view controller
        UIActivityViewController *activityVC = [[UIActivityViewController alloc]
            initWithActivityItems:activityItems
            applicationActivities:nil];

        // Get the top-most view controller
        UIViewController *rootVC = [UIApplication sharedApplication].keyWindow.rootViewController;
        while (rootVC.presentedViewController) {
            rootVC = rootVC.presentedViewController;
        }

        // For iPad, set the popover presentation controller
        if (activityVC.popoverPresentationController) {
            activityVC.popoverPresentationController.sourceView = rootVC.view;
            activityVC.popoverPresentationController.sourceRect = CGRectMake(
                rootVC.view.bounds.size.width / 2,
                rootVC.view.bounds.size.height / 2,
                0, 0
            );
        }

        [rootVC presentViewController:activityVC animated:YES completion:^{
            printf("Share text dialog presented successfully\n");
        }];
    });

    return true;
    #else
    return false;
    #endif
}

bool DclGodotiOS::share_text_with_image(String text, Ref<Image> image) {
    #if TARGET_OS_IOS
    if (image.is_null() || image->is_empty()) {
        printf("Invalid or empty image\n");
        return false;
    }

    NSString *ns_text = [NSString stringWithUTF8String:text.utf8().get_data()];

    // Get image properties
    int width = image->get_width();
    int height = image->get_height();

    // Convert image to RGBA8 format if needed
    Ref<Image> rgba_image = image;
    if (image->get_format() != Image::FORMAT_RGBA8) {
        rgba_image = image->duplicate();
        rgba_image->convert(Image::FORMAT_RGBA8);
    }

    // Get the raw pixel data
    Vector<uint8_t> data = rgba_image->get_data();
    const uint8_t *pixel_data = data.ptr();

    // Create a CGColorSpace for RGB
    CGColorSpaceRef colorSpace = CGColorSpaceCreateDeviceRGB();

    // Create a CGDataProvider from the pixel data
    CFDataRef dataRef = CFDataCreate(NULL, pixel_data, width * height * 4);
    CGDataProviderRef dataProvider = CGDataProviderCreateWithCFData(dataRef);

    // Create CGImage from the pixel data
    CGImageRef cgImage = CGImageCreate(
        width,                                  // width
        height,                                 // height
        8,                                      // bits per component
        32,                                     // bits per pixel
        width * 4,                              // bytes per row
        colorSpace,                             // color space
        kCGBitmapByteOrderDefault | kCGImageAlphaLast, // bitmap info
        dataProvider,                           // data provider
        NULL,                                   // decode
        false,                                  // should interpolate
        kCGRenderingIntentDefault              // rendering intent
    );

    // Convert CGImage to UIImage
    UIImage *uiImage = [UIImage imageWithCGImage:cgImage];

    // Clean up
    CGImageRelease(cgImage);
    CGDataProviderRelease(dataProvider);
    CFRelease(dataRef);
    CGColorSpaceRelease(colorSpace);

    if (!uiImage) {
        printf("Failed to convert Godot Image to UIImage\n");
        return false;
    }

    dispatch_async(dispatch_get_main_queue(), ^{
        // Create activity items array with text and image
        NSArray *activityItems = @[ns_text, uiImage];

        // Create the activity view controller
        UIActivityViewController *activityVC = [[UIActivityViewController alloc]
            initWithActivityItems:activityItems
            applicationActivities:nil];

        // Get the top-most view controller
        UIViewController *rootVC = [UIApplication sharedApplication].keyWindow.rootViewController;
        while (rootVC.presentedViewController) {
            rootVC = rootVC.presentedViewController;
        }

        // For iPad, set the popover presentation controller
        if (activityVC.popoverPresentationController) {
            activityVC.popoverPresentationController.sourceView = rootVC.view;
            activityVC.popoverPresentationController.sourceRect = CGRectMake(
                rootVC.view.bounds.size.width / 2,
                rootVC.view.bounds.size.height / 2,
                0, 0
            );
        }

        [rootVC presentViewController:activityVC animated:YES completion:^{
            printf("Share text with image dialog presented successfully\n");
        }];
    });

    return true;
    #else
    return false;
    #endif
}

// =============================================================================
// LOCAL NOTIFICATIONS
// =============================================================================

void DclGodotiOS::request_notification_permission() {
    #if TARGET_OS_IOS
    UNUserNotificationCenter *center = [UNUserNotificationCenter currentNotificationCenter];

    [center requestAuthorizationWithOptions:(UNAuthorizationOptionAlert |
                                            UNAuthorizationOptionSound |
                                            UNAuthorizationOptionBadge)
                          completionHandler:^(BOOL granted, NSError * _Nullable error) {
        if (granted) {
            printf("Notification permission granted\n");
        } else {
            printf("Notification permission denied\n");
        }
        if (error) {
            printf("Error requesting notification permission: %s\n", error.localizedDescription.UTF8String);
        }
    }];
    #endif
}

bool DclGodotiOS::has_notification_permission() {
    #if TARGET_OS_IOS
    __block bool hasPermission = false;
    __block bool completed = false;

    UNUserNotificationCenter *center = [UNUserNotificationCenter currentNotificationCenter];
    [center getNotificationSettingsWithCompletionHandler:^(UNNotificationSettings * _Nonnull settings) {
        hasPermission = (settings.authorizationStatus == UNAuthorizationStatusAuthorized);
        completed = true;
    }];

    // Wait for completion (with timeout)
    NSDate *timeout = [NSDate dateWithTimeIntervalSinceNow:1.0];
    while (!completed && [[NSDate date] compare:timeout] == NSOrderedAscending) {
        [[NSRunLoop currentRunLoop] runMode:NSDefaultRunLoopMode beforeDate:[NSDate dateWithTimeIntervalSinceNow:0.01]];
    }

    return hasPermission;
    #else
    return false;
    #endif
}

bool DclGodotiOS::schedule_local_notification(String notification_id, String title, String body, int delay_seconds) {
    #if TARGET_OS_IOS
    // Use safer UTF8 conversion to preserve emoji
    CharString id_utf8 = notification_id.utf8();
    CharString title_utf8 = title.utf8();
    CharString body_utf8 = body.utf8();

    NSString *ns_id = [[NSString alloc] initWithBytes:id_utf8.get_data() length:id_utf8.length() encoding:NSUTF8StringEncoding];
    NSString *ns_title = [[NSString alloc] initWithBytes:title_utf8.get_data() length:title_utf8.length() encoding:NSUTF8StringEncoding];
    NSString *ns_body = [[NSString alloc] initWithBytes:body_utf8.get_data() length:body_utf8.length() encoding:NSUTF8StringEncoding];

    // Create notification content
    UNMutableNotificationContent *content = [[UNMutableNotificationContent alloc] init];
    content.title = ns_title;
    content.body = ns_body;
    content.sound = [UNNotificationSound defaultSound];
    content.badge = @(1);

    // Fetch image blob and deep link from database if available
    NSData *imageBlob = nil;
    NSString *deepLink = nil;
    if (notificationDatabase) {
        imageBlob = [notificationDatabase getNotificationImageBlobWithId:ns_id];
        deepLink = [notificationDatabase getNotificationDeepLinkWithId:ns_id];
    }

    // Store deep link in userInfo so it can be retrieved when notification is tapped
    if (deepLink && deepLink.length > 0) {
        content.userInfo = @{@"deep_link": deepLink};
        NSLog(@"[NOTIFICATION] Setting deep link in userInfo: %@", deepLink);
    }

    // Add image attachment if available
    if (imageBlob && imageBlob.length > 0) {
        // Save image to temporary file
        NSString *tempDir = NSTemporaryDirectory();
        NSString *imagePath = [tempDir stringByAppendingPathComponent:[NSString stringWithFormat:@"%@.jpg", ns_id]];
        NSError *writeError = nil;

        if ([imageBlob writeToFile:imagePath options:NSDataWritingAtomic error:&writeError]) {
            NSURL *imageURL = [NSURL fileURLWithPath:imagePath];
            NSError *attachmentError = nil;

            // Options to show thumbnail in collapsed notification
            NSDictionary *options = @{
                UNNotificationAttachmentOptionsThumbnailHiddenKey: @NO,
                UNNotificationAttachmentOptionsThumbnailClippingRectKey: [NSValue valueWithCGRect:CGRectMake(0.0, 0.0, 1.0, 1.0)]
            };

            UNNotificationAttachment *attachment = [UNNotificationAttachment
                attachmentWithIdentifier:@"image"
                URL:imageURL
                options:options
                error:&attachmentError];

            if (attachment && !attachmentError) {
                content.attachments = @[attachment];
                printf("Image attachment added to notification: %s\n", notification_id.utf8().get_data());
            } else if (attachmentError) {
                printf("Error creating attachment: %s\n", attachmentError.localizedDescription.UTF8String);
            }
        } else if (writeError) {
            printf("Error writing image to temp file: %s\n", writeError.localizedDescription.UTF8String);
        }
    }

    // Create trigger (fire after delay_seconds)
    UNTimeIntervalNotificationTrigger *trigger = [UNTimeIntervalNotificationTrigger
        triggerWithTimeInterval:delay_seconds
        repeats:NO];

    // Create notification request
    UNNotificationRequest *request = [UNNotificationRequest
        requestWithIdentifier:ns_id
        content:content
        trigger:trigger];

    // Schedule notification
    UNUserNotificationCenter *center = [UNUserNotificationCenter currentNotificationCenter];
    [center addNotificationRequest:request withCompletionHandler:^(NSError * _Nullable error) {
        if (error) {
            printf("Error scheduling local notification: %s\n", error.localizedDescription.UTF8String);
        } else {
            printf("Local notification scheduled: id=%s, delay=%ds, hasImage=%d\n",
                   notification_id.utf8().get_data(), delay_seconds, (imageBlob != nil && imageBlob.length > 0));
        }
    }];

    return true;
    #else
    return false;
    #endif
}

bool DclGodotiOS::cancel_local_notification(String notification_id) {
    #if TARGET_OS_IOS
    NSString *ns_id = [NSString stringWithUTF8String:notification_id.utf8().get_data()];

    UNUserNotificationCenter *center = [UNUserNotificationCenter currentNotificationCenter];

    // Remove pending notification
    [center removePendingNotificationRequestsWithIdentifiers:@[ns_id]];

    // Also remove delivered notification from notification center
    [center removeDeliveredNotificationsWithIdentifiers:@[ns_id]];

    printf("Local notification cancelled: id=%s\n", notification_id.utf8().get_data());
    return true;
    #else
    return false;
    #endif
}

bool DclGodotiOS::cancel_all_local_notifications() {
    #if TARGET_OS_IOS
    UNUserNotificationCenter *center = [UNUserNotificationCenter currentNotificationCenter];

    // Remove all pending notifications
    [center removeAllPendingNotificationRequests];

    // Remove all delivered notifications
    [center removeAllDeliveredNotifications];

    printf("All local notifications cancelled\n");
    return true;
    #else
    return false;
    #endif
}

void DclGodotiOS::clear_badge_number() {
    #if TARGET_OS_IOS
    dispatch_async(dispatch_get_main_queue(), ^{
        // Clear the badge number
        [UIApplication sharedApplication].applicationIconBadgeNumber = 0;

        // Also remove all delivered notifications from notification center
        UNUserNotificationCenter *center = [UNUserNotificationCenter currentNotificationCenter];
        [center removeAllDeliveredNotifications];

        printf("Badge number cleared and delivered notifications removed\n");
    });
    #endif
}

// =============================================================================
// DATABASE API - Unified notification queue management (Phase 3)
// =============================================================================

bool DclGodotiOS::db_insert_notification(String id, String title, String body, int64_t trigger_timestamp, int is_scheduled, String data, String image_base64) {
    #if TARGET_OS_IOS
    CharString id_utf8 = id.utf8();
    CharString title_utf8 = title.utf8();
    printf("iOS db_insert_notification called: id=%s, title=%s, trigger_ts=%lld, has_image=%d\n",
           id_utf8.get_data(), title_utf8.get_data(), trigger_timestamp, !image_base64.is_empty());

    if (!notificationDatabase) {
        printf("ERROR: Notification database not initialized\n");
        return false;
    }
    printf("Notification database is initialized\n");

    // Use safer UTF8 conversion to preserve emoji
    CharString body_utf8 = body.utf8();

    NSString *ns_id = [[NSString alloc] initWithBytes:id_utf8.get_data() length:id_utf8.length() encoding:NSUTF8StringEncoding];
    NSString *ns_title = [[NSString alloc] initWithBytes:title_utf8.get_data() length:title_utf8.length() encoding:NSUTF8StringEncoding];
    NSString *ns_body = [[NSString alloc] initWithBytes:body_utf8.get_data() length:body_utf8.length() encoding:NSUTF8StringEncoding];

    NSString *ns_data = nil;
    if (!data.is_empty()) {
        CharString data_utf8 = data.utf8();
        ns_data = [[NSString alloc] initWithBytes:data_utf8.get_data() length:data_utf8.length() encoding:NSUTF8StringEncoding];
    }

    // Decode base64 image if provided
    NSData *imageBlob = nil;
    if (!image_base64.is_empty()) {
        printf("Decoding base64 image (length: %d)...\n", (int)image_base64.length());
        CharString image_utf8 = image_base64.utf8();
        NSString *ns_image_base64 = [[NSString alloc] initWithBytes:image_utf8.get_data() length:image_utf8.length() encoding:NSUTF8StringEncoding];
        imageBlob = [[NSData alloc] initWithBase64EncodedString:ns_image_base64 options:0];
        if (!imageBlob) {
            printf("ERROR: Failed to decode base64 image data\n");
        } else {
            printf("Image decoded successfully: %lu bytes\n", (unsigned long)[imageBlob length]);
        }
    }

    printf("Calling insertNotificationWithId on database...\n");
    BOOL result = [notificationDatabase insertNotificationWithId:ns_id
                                                           title:ns_title
                                                            body:ns_body
                                                 triggerTimestamp:trigger_timestamp
                                                     isScheduled:is_scheduled
                                                            data:ns_data
                                                       imageBlob:imageBlob];

    printf("Database insert result: %s\n", result ? "SUCCESS" : "FAILURE");
    return result;
    #else
    return false;
    #endif
}

bool DclGodotiOS::db_update_notification(String id, Dictionary updates) {
    #if TARGET_OS_IOS
    if (!notificationDatabase) {
        printf("Notification database not initialized\n");
        return false;
    }

    NSString *ns_id = [NSString stringWithUTF8String:id.utf8().get_data()];

    // Convert Godot Dictionary to NSDictionary
    NSMutableDictionary *ns_updates = [NSMutableDictionary dictionary];
    Array keys = updates.keys();
    for (int i = 0; i < keys.size(); i++) {
        String key = keys[i];
        Variant value = updates[key];

        NSString *ns_key = [NSString stringWithUTF8String:key.utf8().get_data()];

        if (value.get_type() == Variant::STRING) {
            ns_updates[ns_key] = [NSString stringWithUTF8String:String(value).utf8().get_data()];
        } else if (value.get_type() == Variant::INT) {
            ns_updates[ns_key] = @((long long)value);
        }
    }

    return [notificationDatabase updateNotificationWithId:ns_id updates:ns_updates];
    #else
    return false;
    #endif
}

bool DclGodotiOS::db_delete_notification(String id) {
    #if TARGET_OS_IOS
    if (!notificationDatabase) {
        printf("Notification database not initialized\n");
        return false;
    }

    NSString *ns_id = [NSString stringWithUTF8String:id.utf8().get_data()];
    return [notificationDatabase deleteNotificationWithId:ns_id];
    #else
    return false;
    #endif
}

TypedArray<Dictionary> DclGodotiOS::db_query_notifications(String where_clause, String order_by, int limit) {
    TypedArray<Dictionary> result;

    #if TARGET_OS_IOS
    if (!notificationDatabase) {
        printf("Notification database not initialized\n");
        return result;
    }

    NSString *ns_where = where_clause.is_empty() ? @"" : [NSString stringWithUTF8String:where_clause.utf8().get_data()];
    NSString *ns_order = order_by.is_empty() ? @"" : [NSString stringWithUTF8String:order_by.utf8().get_data()];

    NSArray<NSDictionary *> *ns_results = [notificationDatabase queryNotificationsWithWhere:ns_where
                                                                                    orderBy:ns_order
                                                                                      limit:limit];

    // Convert NSArray to Godot TypedArray
    for (NSDictionary *ns_dict in ns_results) {
        Dictionary dict;

        // Convert NSDictionary to Godot Dictionary
        for (NSString *key in ns_dict.allKeys) {
            NSObject *objc_value = ns_dict[key];
            String str_key = String([key UTF8String]);

            if ([objc_value isKindOfClass:[NSString class]]) {
                dict[str_key] = String([(NSString *)objc_value UTF8String]);
            } else if ([objc_value isKindOfClass:[NSNumber class]]) {
                NSNumber *num = (NSNumber *)objc_value;
                // Check if it's a long long or int
                if (strcmp([num objCType], @encode(long long)) == 0) {
                    dict[str_key] = (int64_t)[num longLongValue];
                } else {
                    dict[str_key] = [num intValue];
                }
            }
        }

        result.append(dict);
    }
    #endif

    return result;
}

int DclGodotiOS::db_count_notifications(String where_clause) {
    #if TARGET_OS_IOS
    if (!notificationDatabase) {
        printf("Notification database not initialized\n");
        return 0;
    }

    NSString *ns_where = where_clause.is_empty() ? @"" : [NSString stringWithUTF8String:where_clause.utf8().get_data()];
    return [notificationDatabase countNotificationsWithWhere:ns_where];
    #else
    return 0;
    #endif
}

int DclGodotiOS::db_clear_expired(int64_t current_timestamp) {
    #if TARGET_OS_IOS
    if (!notificationDatabase) {
        printf("Notification database not initialized\n");
        return 0;
    }

    return [notificationDatabase clearExpiredWithTimestamp:current_timestamp];
    #else
    return 0;
    #endif
}

bool DclGodotiOS::db_mark_scheduled(String id, bool is_scheduled) {
    #if TARGET_OS_IOS
    if (!notificationDatabase) {
        printf("Notification database not initialized\n");
        return false;
    }

    NSString *ns_id = [NSString stringWithUTF8String:id.utf8().get_data()];
    return [notificationDatabase markScheduledWithId:ns_id isScheduled:is_scheduled];
    #else
    return false;
    #endif
}

Dictionary DclGodotiOS::db_get_notification(String id) {
    Dictionary result;

    #if TARGET_OS_IOS
    if (!notificationDatabase) {
        printf("Notification database not initialized\n");
        return result;
    }

    NSString *ns_id = [NSString stringWithUTF8String:id.utf8().get_data()];
    NSDictionary *ns_dict = [notificationDatabase getNotificationWithId:ns_id];

    // Convert NSDictionary to Godot Dictionary
    for (NSString *key in ns_dict.allKeys) {
        NSObject *objc_value = ns_dict[key];
        String str_key = String([key UTF8String]);

        if ([objc_value isKindOfClass:[NSString class]]) {
            result[str_key] = String([(NSString *)objc_value UTF8String]);
        } else if ([objc_value isKindOfClass:[NSNumber class]]) {
            NSNumber *num = (NSNumber *)objc_value;
            if (strcmp([num objCType], @encode(long long)) == 0) {
                result[str_key] = (int64_t)[num longLongValue];
            } else {
                result[str_key] = [num intValue];
            }
        }
    }
    #endif

    return result;
}

int DclGodotiOS::db_clear_all() {
    #if TARGET_OS_IOS
    if (!notificationDatabase) {
        printf("Notification database not initialized\n");
        return 0;
    }

    return [notificationDatabase clearAll];
    #else
    return 0;
    #endif
}

String DclGodotiOS::db_get_notification_image_blob(String id) {
    #if TARGET_OS_IOS
    if (!notificationDatabase) {
        printf("Notification database not initialized\n");
        return "";
    }

    NSString *ns_id = [NSString stringWithUTF8String:id.utf8().get_data()];
    NSData *imageBlob = [notificationDatabase getNotificationImageBlobWithId:ns_id];

    if (!imageBlob || imageBlob.length == 0) {
        return "";
    }

    // Convert NSData to base64 string
    NSString *base64String = [imageBlob base64EncodedStringWithOptions:0];
    return String([base64String UTF8String]);
    #else
    return "";
    #endif
}

// =============================================================================
// OS NOTIFICATION API - Renamed for clarity (Phase 3)
// =============================================================================

bool DclGodotiOS::os_schedule_notification(String notification_id, String title, String body, int delay_seconds) {
    // This is the same as the existing schedule_local_notification
    return schedule_local_notification(notification_id, title, body, delay_seconds);
}

bool DclGodotiOS::os_cancel_notification(String notification_id) {
    // This is the same as the existing cancel_local_notification
    return cancel_local_notification(notification_id);
}

PackedStringArray DclGodotiOS::os_get_scheduled_ids() {
    PackedStringArray result;

    #if TARGET_OS_IOS
    // Query the UNUserNotificationCenter for pending notification requests
    __block PackedStringArray pending_ids;
    __block bool completed = false;

    UNUserNotificationCenter *center = [UNUserNotificationCenter currentNotificationCenter];
    [center getPendingNotificationRequestsWithCompletionHandler:^(NSArray<UNNotificationRequest *> * _Nonnull requests) {
        for (UNNotificationRequest *request in requests) {
            pending_ids.append(String([request.identifier UTF8String]));
        }
        completed = true;
    }];

    // Wait for completion (with timeout)
    NSDate *timeout = [NSDate dateWithTimeIntervalSinceNow:1.0];
    while (!completed && [[NSDate date] compare:timeout] == NSOrderedAscending) {
        [[NSRunLoop currentRunLoop] runMode:NSDefaultRunLoopMode beforeDate:[NSDate dateWithTimeIntervalSinceNow:0.01]];
    }

    result = pending_ids;
    #endif

    return result;
}

// =============================================================================
// AVPLAYER API - Hardware-accelerated video playback with zero-copy GPU textures
// =============================================================================

int DclGodotiOS::createAVPlayer() {
    #if TARGET_OS_IOS
    int playerId = nextAvPlayerId++;
    AVPlayerWrapper *player = [[AVPlayerWrapper alloc] initWithId:playerId];
    avPlayers[playerId] = player;
    printf("Created AVPlayer with ID: %d\n", playerId);
    return playerId;
    #else
    return -1;
    #endif
}

void DclGodotiOS::avPlayerRelease(int player_id) {
    #if TARGET_OS_IOS
    auto it = avPlayers.find(player_id);
    if (it != avPlayers.end()) {
        AVPlayerWrapper *player = it->second;
        [player releasePlayer];
        avPlayers.erase(it);
        printf("Released AVPlayer with ID: %d\n", player_id);
    }
    #endif
}

int DclGodotiOS::avPlayerInitSurface(int player_id, int width, int height) {
    #if TARGET_OS_IOS
    auto it = avPlayers.find(player_id);
    if (it != avPlayers.end()) {
        return [it->second initializeSurfaceWithWidth:width height:height];
    }
    #endif
    return 0;
}

bool DclGodotiOS::avPlayerSetSourceUrl(int player_id, String url) {
    #if TARGET_OS_IOS
    auto it = avPlayers.find(player_id);
    if (it != avPlayers.end()) {
        NSString *ns_url = [NSString stringWithUTF8String:url.utf8().get_data()];
        return [it->second setSourceURL:ns_url];
    }
    #endif
    return false;
}

bool DclGodotiOS::avPlayerSetSourceLocal(int player_id, String file_path) {
    #if TARGET_OS_IOS
    auto it = avPlayers.find(player_id);
    if (it != avPlayers.end()) {
        NSString *ns_path = [NSString stringWithUTF8String:file_path.utf8().get_data()];
        return [it->second setSourceLocal:ns_path];
    }
    #endif
    return false;
}

void DclGodotiOS::avPlayerPlay(int player_id) {
    #if TARGET_OS_IOS
    auto it = avPlayers.find(player_id);
    if (it != avPlayers.end()) {
        [it->second play];
    }
    #endif
}

void DclGodotiOS::avPlayerPause(int player_id) {
    #if TARGET_OS_IOS
    auto it = avPlayers.find(player_id);
    if (it != avPlayers.end()) {
        [it->second pause];
    }
    #endif
}

void DclGodotiOS::avPlayerStop(int player_id) {
    #if TARGET_OS_IOS
    auto it = avPlayers.find(player_id);
    if (it != avPlayers.end()) {
        [it->second stop];
    }
    #endif
}

void DclGodotiOS::avPlayerSetPosition(int player_id, float position_sec) {
    #if TARGET_OS_IOS
    auto it = avPlayers.find(player_id);
    if (it != avPlayers.end()) {
        [it->second setPosition:position_sec];
    }
    #endif
}

float DclGodotiOS::avPlayerGetPosition(int player_id) {
    #if TARGET_OS_IOS
    auto it = avPlayers.find(player_id);
    if (it != avPlayers.end()) {
        return [it->second getPosition];
    }
    #endif
    return 0.0f;
}

float DclGodotiOS::avPlayerGetDuration(int player_id) {
    #if TARGET_OS_IOS
    auto it = avPlayers.find(player_id);
    if (it != avPlayers.end()) {
        return [it->second getDuration];
    }
    #endif
    return 0.0f;
}

bool DclGodotiOS::avPlayerIsPlaying(int player_id) {
    #if TARGET_OS_IOS
    auto it = avPlayers.find(player_id);
    if (it != avPlayers.end()) {
        return [it->second isPlaying];
    }
    #endif
    return false;
}

int DclGodotiOS::avPlayerGetVideoWidth(int player_id) {
    #if TARGET_OS_IOS
    auto it = avPlayers.find(player_id);
    if (it != avPlayers.end()) {
        return [it->second videoWidth];
    }
    #endif
    return 0;
}

int DclGodotiOS::avPlayerGetVideoHeight(int player_id) {
    #if TARGET_OS_IOS
    auto it = avPlayers.find(player_id);
    if (it != avPlayers.end()) {
        return [it->second videoHeight];
    }
    #endif
    return 0;
}

bool DclGodotiOS::avPlayerHasVideoSizeChanged(int player_id) {
    #if TARGET_OS_IOS
    auto it = avPlayers.find(player_id);
    if (it != avPlayers.end()) {
        BOOL changed = [it->second hasVideoSizeChanged];
        if (changed) {
            [it->second clearVideoSizeChangedFlag];
        }
        return changed;
    }
    #endif
    return false;
}

int DclGodotiOS::avPlayerGetTextureWidth(int player_id) {
    #if TARGET_OS_IOS
    auto it = avPlayers.find(player_id);
    if (it != avPlayers.end()) {
        return [it->second textureWidth];
    }
    #endif
    return 0;
}

int DclGodotiOS::avPlayerGetTextureHeight(int player_id) {
    #if TARGET_OS_IOS
    auto it = avPlayers.find(player_id);
    if (it != avPlayers.end()) {
        return [it->second textureHeight];
    }
    #endif
    return 0;
}

void DclGodotiOS::avPlayerSetVolume(int player_id, float volume) {
    #if TARGET_OS_IOS
    auto it = avPlayers.find(player_id);
    if (it != avPlayers.end()) {
        [it->second setVolume:volume];
    }
    #endif
}

float DclGodotiOS::avPlayerGetVolume(int player_id) {
    #if TARGET_OS_IOS
    auto it = avPlayers.find(player_id);
    if (it != avPlayers.end()) {
        return [it->second volume];
    }
    #endif
    return 1.0f;
}

void DclGodotiOS::avPlayerSetLooping(int player_id, bool loop) {
    #if TARGET_OS_IOS
    auto it = avPlayers.find(player_id);
    if (it != avPlayers.end()) {
        [it->second setIsLooping:loop];
    }
    #endif
}

void DclGodotiOS::avPlayerSetPlaybackRate(int player_id, float rate) {
    #if TARGET_OS_IOS
    auto it = avPlayers.find(player_id);
    if (it != avPlayers.end()) {
        [it->second setPlaybackRate:rate];
    }
    #endif
}

bool DclGodotiOS::avPlayerHasNewPixelBuffer(int player_id) {
    #if TARGET_OS_IOS
    auto it = avPlayers.find(player_id);
    if (it != avPlayers.end()) {
        return [it->second hasNewPixelBuffer];
    }
    #endif
    return false;
}

uint64_t DclGodotiOS::avPlayerAcquireIOSurfacePtr(int player_id) {
    #if TARGET_OS_IOS
    auto it = avPlayers.find(player_id);
    if (it != avPlayers.end()) {
        return [it->second acquireIOSurfacePtr];
    }
    #endif
    return 0;
}

String DclGodotiOS::avPlayerGetInfo(int player_id) {
    #if TARGET_OS_IOS
    auto it = avPlayers.find(player_id);
    if (it != avPlayers.end()) {
        NSString *info = [it->second getPlayerInfo];
        return String([info UTF8String]);
    }
    #endif
    return "Player not found";
}

DclGodotiOS *DclGodotiOS::get_singleton() {
    return instance;
}

DclGodotiOS::DclGodotiOS() {
    instance = this;
    authSession = nullptr;
    authDelegate = nullptr;
    calendarDelegate = nullptr;
    nextAvPlayerId = 1;

    #if TARGET_OS_IOS
    // Initialize notification database
    notificationDatabase = [[NotificationDatabase alloc] init];
    printf("Notification database initialized\n");
    printf("AVPlayer API initialized\n");
    #else
    notificationDatabase = nullptr;
    #endif
}

DclGodotiOS::~DclGodotiOS() {
    instance = NULL;
    authSession = nullptr;
    authDelegate = nullptr;
    calendarDelegate = nullptr;

    #if TARGET_OS_IOS
    // Cleanup AVPlayer instances
    for (auto& pair : avPlayers) {
        [pair.second releasePlayer];
    }
    avPlayers.clear();

    if (notificationDatabase) {
        notificationDatabase = nullptr;
    }
    #endif
}
