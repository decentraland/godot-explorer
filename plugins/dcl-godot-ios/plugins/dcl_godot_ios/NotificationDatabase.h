#import <Foundation/Foundation.h>
#import <sqlite3.h>

NS_ASSUME_NONNULL_BEGIN

/**
 * Objective-C wrapper for SQLite notification database.
 * Implements the unified database API as specified in TASK.md Phase 3.
 */
@interface NotificationDatabase : NSObject

- (instancetype)init;
- (void)dealloc;

/**
 * Insert or replace a notification in the database.
 *
 * @param notificationId Unique notification ID
 * @param title Notification title
 * @param body Notification body
 * @param triggerTimestamp Unix timestamp (seconds) when notification should fire
 * @param isScheduled Whether notification is currently scheduled with OS (0 or 1)
 * @param data Optional JSON string for extra metadata (can be nil)
 * @param imageBlob Optional image data as NSData (can be nil)
 * @return YES if successful
 */
- (BOOL)insertNotificationWithId:(NSString *)notificationId
                           title:(NSString *)title
                            body:(NSString *)body
                 triggerTimestamp:(long long)triggerTimestamp
                     isScheduled:(int)isScheduled
                            data:(NSString * _Nullable)data
                       imageBlob:(NSData * _Nullable)imageBlob;

/**
 * Update notification fields.
 *
 * @param notificationId Notification ID to update
 * @param updates Dictionary with field names and new values
 * @return YES if successful
 */
- (BOOL)updateNotificationWithId:(NSString *)notificationId
                         updates:(NSDictionary *)updates;

/**
 * Delete a notification by ID.
 *
 * @param notificationId Notification ID to delete
 * @return YES if successful
 */
- (BOOL)deleteNotificationWithId:(NSString *)notificationId;

/**
 * Query notifications with filters.
 *
 * @param whereClause SQL WHERE clause (without "WHERE" keyword), e.g. "is_scheduled = 0"
 * @param orderBy SQL ORDER BY clause (without "ORDER BY" keyword), e.g. "trigger_timestamp ASC"
 * @param limit Maximum number of results, or -1 for no limit
 * @return Array of notification dictionaries
 */
- (NSArray<NSDictionary *> *)queryNotificationsWithWhere:(NSString *)whereClause
                                                 orderBy:(NSString *)orderBy
                                                   limit:(int)limit;

/**
 * Get count of notifications matching filter.
 *
 * @param whereClause SQL WHERE clause (without "WHERE" keyword)
 * @return Count of matching notifications
 */
- (int)countNotificationsWithWhere:(NSString *)whereClause;

/**
 * Clear expired notifications (trigger_timestamp < current_time).
 *
 * @param currentTimestamp Current Unix timestamp (seconds)
 * @return Number of deleted notifications
 */
- (int)clearExpiredWithTimestamp:(long long)currentTimestamp;

/**
 * Mark notification as scheduled/unscheduled.
 *
 * @param notificationId Notification ID
 * @param isScheduled YES if scheduled with OS, NO otherwise
 * @return YES if successful
 */
- (BOOL)markScheduledWithId:(NSString *)notificationId
                isScheduled:(BOOL)isScheduled;

/**
 * Get a single notification by ID.
 *
 * @param notificationId Notification ID
 * @return Dictionary with notification data, or empty dictionary if not found
 */
- (NSDictionary *)getNotificationWithId:(NSString *)notificationId;

/**
 * Clear all notifications from database.
 *
 * @return Number of deleted notifications
 */
- (int)clearAll;

/**
 * Get the image blob for a specific notification.
 * This is separate from queryNotifications() to avoid loading images into memory unnecessarily.
 *
 * @param notificationId Notification ID
 * @return NSData with image data, or nil if no image or not found
 */
- (NSData * _Nullable)getNotificationImageBlobWithId:(NSString *)notificationId;

@end

NS_ASSUME_NONNULL_END
