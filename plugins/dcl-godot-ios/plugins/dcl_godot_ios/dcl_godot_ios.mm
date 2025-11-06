#include "dcl_godot_ios.h"
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

void DclGodotiOS::_bind_methods() {
    ClassDB::bind_method(D_METHOD("print_version"), &DclGodotiOS::print_version);
    ClassDB::bind_method(D_METHOD("open_auth_url", "url"), &DclGodotiOS::open_auth_url);
    ClassDB::bind_method(D_METHOD("get_mobile_device_info"), &DclGodotiOS::get_mobile_device_info);
    ClassDB::bind_method(D_METHOD("get_mobile_metrics"), &DclGodotiOS::get_mobile_metrics);
    ClassDB::bind_method(D_METHOD("add_calendar_event", "title", "description", "start_time", "end_time", "location"), &DclGodotiOS::add_calendar_event);
    ClassDB::bind_method(D_METHOD("share_text", "text"), &DclGodotiOS::share_text);
    ClassDB::bind_method(D_METHOD("share_text_with_image", "text", "image"), &DclGodotiOS::share_text_with_image);

    // Local notifications
    ClassDB::bind_method(D_METHOD("request_notification_permission"), &DclGodotiOS::request_notification_permission);
    ClassDB::bind_method(D_METHOD("has_notification_permission"), &DclGodotiOS::has_notification_permission);
    ClassDB::bind_method(D_METHOD("schedule_local_notification", "notification_id", "title", "body", "delay_seconds"), &DclGodotiOS::schedule_local_notification);
    ClassDB::bind_method(D_METHOD("cancel_local_notification", "notification_id"), &DclGodotiOS::cancel_local_notification);
    ClassDB::bind_method(D_METHOD("cancel_all_local_notifications"), &DclGodotiOS::cancel_all_local_notifications);
    ClassDB::bind_method(D_METHOD("clear_badge_number"), &DclGodotiOS::clear_badge_number);
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
    NSString *ns_id = [NSString stringWithUTF8String:notification_id.utf8().get_data()];
    NSString *ns_title = [NSString stringWithUTF8String:title.utf8().get_data()];
    NSString *ns_body = [NSString stringWithUTF8String:body.utf8().get_data()];

    // Create notification content
    UNMutableNotificationContent *content = [[UNMutableNotificationContent alloc] init];
    content.title = ns_title;
    content.body = ns_body;
    content.sound = [UNNotificationSound defaultSound];
    content.badge = @(1);

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
            printf("Local notification scheduled: id=%s, delay=%ds\n",
                   notification_id.utf8().get_data(), delay_seconds);
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

DclGodotiOS *DclGodotiOS::get_singleton() {
    return instance;
}

DclGodotiOS::DclGodotiOS() {
    instance = this;
    authSession = nullptr;
    authDelegate = nullptr;
    calendarDelegate = nullptr;
}

DclGodotiOS::~DclGodotiOS() {
    instance = NULL;
    authSession = nullptr;
    authDelegate = nullptr;
    calendarDelegate = nullptr;
}
