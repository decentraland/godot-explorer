//
// Notification service for handling local notification taps and deep links
//

#import "notification_service.h"
#import "dcl_godot_ios.h"
#import <UserNotifications/UserNotifications.h>

static bool notification_service_initialized = false;

struct NotificationServiceInitializer {
    NotificationServiceInitializer() {
        if (!notification_service_initialized) {
            // Set the delegate on the notification center
            [UNUserNotificationCenter currentNotificationCenter].delegate = [NotificationService shared];
            notification_service_initialized = true;
            NSLog(@"[NOTIFICATION] NotificationService initialized and set as delegate");
        }
    }
};
static NotificationServiceInitializer initializer;

// C function to force initialization
void force_notification_service_initialization() {
    if (!notification_service_initialized) {
        [UNUserNotificationCenter currentNotificationCenter].delegate = [NotificationService shared];
        notification_service_initialized = true;
        NSLog(@"[NOTIFICATION] NotificationService force initialized");
    }
}

@implementation NotificationService

- (instancetype)init {
    self = [super init];
    NSLog(@"[NOTIFICATION] NotificationService initialized!");
    return self;
}

+ (instancetype)shared {
    static NotificationService* sharedInstance = nil;
    static dispatch_once_t onceToken;
    dispatch_once(&onceToken, ^{
        sharedInstance = [[NotificationService alloc] init];
    });
    return sharedInstance;
}

// Called when user taps on a notification (app in background or killed)
- (void)userNotificationCenter:(UNUserNotificationCenter *)center
didReceiveNotificationResponse:(UNNotificationResponse *)response
         withCompletionHandler:(void (^)(void))completionHandler {

    NSLog(@"[NOTIFICATION] Notification tapped: %@", response.notification.request.identifier);

    NSDictionary *userInfo = response.notification.request.content.userInfo;
    NSString *deepLink = userInfo[@"deep_link"];

    if (deepLink && deepLink.length > 0) {
        NSLog(@"[NOTIFICATION] Deep link found in notification: %@", deepLink);
        // Emit the deep link so the app can handle it
        DclGodotiOS::emit_deeplink_received(String([deepLink UTF8String]));
    } else {
        NSLog(@"[NOTIFICATION] No deep link in notification userInfo");
    }

    completionHandler();
}

// Called when notification is received while app is in foreground
- (void)userNotificationCenter:(UNUserNotificationCenter *)center
       willPresentNotification:(UNNotification *)notification
         withCompletionHandler:(void (^)(UNNotificationPresentationOptions))completionHandler {

    NSLog(@"[NOTIFICATION] Notification received in foreground: %@", notification.request.identifier);

    // Show the notification even when app is in foreground
    if (@available(iOS 14.0, *)) {
        completionHandler(UNNotificationPresentationOptionBanner | UNNotificationPresentationOptionSound | UNNotificationPresentationOptionBadge);
    } else {
        completionHandler(UNNotificationPresentationOptionAlert | UNNotificationPresentationOptionSound | UNNotificationPresentationOptionBadge);
    }
}

@end
