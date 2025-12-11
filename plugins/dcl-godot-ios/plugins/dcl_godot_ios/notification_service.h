//
// Notification service for handling local notification taps and deep links
//

#ifndef notification_service_h
#define notification_service_h

#ifdef __OBJC__
#import <UIKit/UIKit.h>
#import <UserNotifications/UserNotifications.h>

@interface NotificationService : NSObject<UNUserNotificationCenterDelegate>

+ (instancetype) shared;

@end
#endif

// C++ function to ensure the NotificationService initializer runs
#ifdef __cplusplus
extern "C" {
#endif

void force_notification_service_initialization();

#ifdef __cplusplus
}
#endif

#endif /* notification_service_h */
