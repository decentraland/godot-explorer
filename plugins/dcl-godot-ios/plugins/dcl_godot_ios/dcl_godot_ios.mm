#include "dcl_godot_ios.h"
#include "core/version.h"
#import <SafariServices/SafariServices.h>
#import <AuthenticationServices/AuthenticationServices.h>
#import <UIKit/UIKit.h>
#import <Foundation/Foundation.h>
#import <Network/Network.h>
#import <sys/sysctl.h>
#import <mach/mach.h>

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
    ClassDB::bind_method(D_METHOD("get_mobile_device_info"), &DclGodotiOS::get_mobile_device_info);
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
    // Static variables for battery drain calculation
    static float initial_battery_level = -1.0f;
    static NSDate *initial_battery_timestamp = nil;

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
    info["thermal_state"] = String(thermalState.UTF8String);

    // Get battery level and calculate drain
    float current_battery_level = [[UIDevice currentDevice] batteryLevel] * 100.0f; // 0-100
    info["battery_level"] = current_battery_level;

    if (initial_battery_level < 0.0f) {
        // First call - initialize
        initial_battery_level = current_battery_level;
        initial_battery_timestamp = [NSDate date];
        info["battery_drain_pct_per_hour"] = 0.0f;
    } else {
        // Calculate drain rate
        NSTimeInterval elapsed_seconds = [[NSDate date] timeIntervalSinceDate:initial_battery_timestamp];
        float elapsed_hours = elapsed_seconds / 3600.0f;

        if (elapsed_hours > 0.0f) {
            float drain_pct_per_hour = (initial_battery_level - current_battery_level) / elapsed_hours;
            info["battery_drain_pct_per_hour"] = drain_pct_per_hour;
        } else {
            info["battery_drain_pct_per_hour"] = 0.0f;
        }
    }

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

    // Get RAM consumption using phys_footprint (what Xcode uses)
    struct task_vm_info vm_info;
    mach_msg_type_number_t vm_info_count = TASK_VM_INFO_COUNT;
    if (task_info(mach_task_self(), TASK_VM_INFO, (task_info_t)&vm_info, &vm_info_count) == KERN_SUCCESS) {
        // phys_footprint is the actual physical memory used (what Xcode shows)
        info["memory_usage"] = (int)(vm_info.phys_footprint / (1024 * 1024));
    } else {
        info["memory_usage"] = 0;
    }

    // Get network type using NWPathMonitor (simplified synchronous check)
    __block String network_type = "Unknown";
    dispatch_semaphore_t semaphore = dispatch_semaphore_create(0);

    nw_path_monitor_t monitor = nw_path_monitor_create();
    nw_path_monitor_set_queue(monitor, dispatch_get_global_queue(DISPATCH_QUEUE_PRIORITY_DEFAULT, 0));
    nw_path_monitor_set_update_handler(monitor, ^(nw_path_t path) {
        if (nw_path_get_status(path) == nw_path_status_satisfied) {
            if (nw_path_uses_interface_type(path, nw_interface_type_wifi)) {
                network_type = "WiFi";
            } else if (nw_path_uses_interface_type(path, nw_interface_type_cellular)) {
                // Try to determine cellular generation (simplified)
                network_type = "Cellular";
            } else if (nw_path_uses_interface_type(path, nw_interface_type_wired)) {
                network_type = "Wired";
            } else {
                network_type = "Other";
            }
        } else {
            network_type = "No Connection";
        }
        dispatch_semaphore_signal(semaphore);
    });
    nw_path_monitor_start(monitor);

    // Wait for network check with timeout
    dispatch_semaphore_wait(semaphore, dispatch_time(DISPATCH_TIME_NOW, 1 * NSEC_PER_SEC));
    nw_path_monitor_cancel(monitor);

    info["network_type"] = network_type;
    info["network_speed_mbps"] = 0.0f; // To be calculated later

    #endif

    return info;
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
