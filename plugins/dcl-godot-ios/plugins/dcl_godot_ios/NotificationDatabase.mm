#import "NotificationDatabase.h"

@interface NotificationDatabase ()
@property (nonatomic, assign) sqlite3 *database;
@end

@implementation NotificationDatabase

- (instancetype)init {
    self = [super init];
    if (self) {
        [self openDatabase];
        [self createTables];
    }
    return self;
}

- (void)dealloc {
    if (_database) {
        sqlite3_close(_database);
        _database = NULL;
    }
}

- (void)openDatabase {
    // Get database file path in app's documents directory
    NSArray *paths = NSSearchPathForDirectoriesInDomains(NSDocumentDirectory, NSUserDomainMask, YES);
    NSString *documentsDirectory = [paths firstObject];
    NSString *dbPath = [documentsDirectory stringByAppendingPathComponent:@"dcl_notifications.db"];

    // Open or create database
    int result = sqlite3_open([dbPath UTF8String], &_database);
    if (result != SQLITE_OK) {
        NSLog(@"Error opening database: %s", sqlite3_errmsg(_database));
        _database = NULL;
    } else {
        NSLog(@"Notification database opened successfully at: %@", dbPath);
    }
}

- (int)getDatabaseVersion {
    if (!_database) return 0;

    sqlite3_stmt *statement;
    const char *sql = "PRAGMA user_version;";

    if (sqlite3_prepare_v2(_database, sql, -1, &statement, NULL) != SQLITE_OK) {
        NSLog(@"Error getting database version: %s", sqlite3_errmsg(_database));
        return 0;
    }

    int version = 0;
    if (sqlite3_step(statement) == SQLITE_ROW) {
        version = sqlite3_column_int(statement, 0);
    }

    sqlite3_finalize(statement);
    return version;
}

- (void)setDatabaseVersion:(int)version {
    if (!_database) return;

    char *errorMsg;
    NSString *sql = [NSString stringWithFormat:@"PRAGMA user_version = %d;", version];

    int result = sqlite3_exec(_database, [sql UTF8String], NULL, NULL, &errorMsg);
    if (result != SQLITE_OK) {
        NSLog(@"Error setting database version: %s", errorMsg);
        sqlite3_free(errorMsg);
    } else {
        NSLog(@"Database version set to: %d", version);
    }
}

- (void)createTables {
    if (!_database) return;

    char *errorMsg;
    int currentVersion = [self getDatabaseVersion];
    const int TARGET_VERSION = 1;

    NSLog(@"Database current version: %d, target version: %d", currentVersion, TARGET_VERSION);

    if (currentVersion == 0) {
        // Fresh database - create schema from scratch
        NSLog(@"Creating fresh database schema...");

        const char *createTableSQL =
            "CREATE TABLE IF NOT EXISTS notifications ("
            "id TEXT PRIMARY KEY,"
            "title TEXT NOT NULL,"
            "body TEXT NOT NULL,"
            "trigger_timestamp INTEGER NOT NULL,"
            "created_timestamp INTEGER NOT NULL,"
            "is_scheduled INTEGER DEFAULT 0,"
            "data TEXT,"
            "image_blob BLOB"
            ");";

        int result = sqlite3_exec(_database, createTableSQL, NULL, NULL, &errorMsg);
        if (result != SQLITE_OK) {
            NSLog(@"Error creating table: %s", errorMsg);
            sqlite3_free(errorMsg);
            return;
        }

        // Create indexes
        const char *createIndex1SQL = "CREATE INDEX IF NOT EXISTS idx_trigger_time ON notifications(trigger_timestamp);";
        result = sqlite3_exec(_database, createIndex1SQL, NULL, NULL, &errorMsg);
        if (result != SQLITE_OK) {
            NSLog(@"Error creating index 1: %s", errorMsg);
            sqlite3_free(errorMsg);
        }

        const char *createIndex2SQL = "CREATE INDEX IF NOT EXISTS idx_scheduled_time ON notifications(is_scheduled, trigger_timestamp);";
        result = sqlite3_exec(_database, createIndex2SQL, NULL, NULL, &errorMsg);
        if (result != SQLITE_OK) {
            NSLog(@"Error creating index 2: %s", errorMsg);
            sqlite3_free(errorMsg);
        }

        [self setDatabaseVersion:TARGET_VERSION];
        NSLog(@"Fresh database created successfully with version %d", TARGET_VERSION);
    } else if (currentVersion < TARGET_VERSION) {
        // Perform migrations
        NSLog(@"Migrating database from version %d to %d...", currentVersion, TARGET_VERSION);

        // Future migrations would go here
        // Example:
        // if (currentVersion == 1) {
        //     // Migration from v1 to v2
        //     NSLog(@"Applying migration v1 -> v2: ...");
        //     const char *migrationSQL = "ALTER TABLE notifications ADD COLUMN new_column TEXT;";
        //     int result = sqlite3_exec(_database, migrationSQL, NULL, NULL, &errorMsg);
        //     if (result != SQLITE_OK) {
        //         NSLog(@"Error in migration: %s", errorMsg);
        //         sqlite3_free(errorMsg);
        //         return;
        //     }
        //     NSLog(@"Migration v1 -> v2 completed successfully");
        // }

        [self setDatabaseVersion:TARGET_VERSION];
        NSLog(@"Database migration completed. Now at version %d", TARGET_VERSION);
    } else {
        NSLog(@"Database already at current version %d", currentVersion);
    }
}

- (BOOL)insertNotificationWithId:(NSString *)notificationId
                           title:(NSString *)title
                            body:(NSString *)body
                 triggerTimestamp:(long long)triggerTimestamp
                     isScheduled:(int)isScheduled
                            data:(NSString *)data
                       imageBlob:(NSData *)imageBlob {
    printf("NotificationDatabase: insertNotification called for id=%s\n", [notificationId UTF8String]);

    if (!_database) {
        printf("NotificationDatabase ERROR: Database is NULL\n");
        return NO;
    }

    const char *sql = "INSERT OR REPLACE INTO notifications (id, title, body, trigger_timestamp, created_timestamp, is_scheduled, data, image_blob) VALUES (?, ?, ?, ?, ?, ?, ?, ?);";

    sqlite3_stmt *statement;
    if (sqlite3_prepare_v2(_database, sql, -1, &statement, NULL) != SQLITE_OK) {
        printf("NotificationDatabase ERROR: Failed to prepare statement: %s\n", sqlite3_errmsg(_database));
        NSLog(@"Error preparing insert statement: %s", sqlite3_errmsg(_database));
        return NO;
    }

    long long currentTimestamp = (long long)[[NSDate date] timeIntervalSince1970];

    sqlite3_bind_text(statement, 1, [notificationId UTF8String], -1, SQLITE_TRANSIENT);
    sqlite3_bind_text(statement, 2, [title UTF8String], -1, SQLITE_TRANSIENT);
    sqlite3_bind_text(statement, 3, [body UTF8String], -1, SQLITE_TRANSIENT);
    sqlite3_bind_int64(statement, 4, triggerTimestamp);
    sqlite3_bind_int64(statement, 5, currentTimestamp);
    sqlite3_bind_int(statement, 6, isScheduled);

    if (data && data.length > 0) {
        sqlite3_bind_text(statement, 7, [data UTF8String], -1, SQLITE_TRANSIENT);
    } else {
        sqlite3_bind_null(statement, 7);
    }

    if (imageBlob && imageBlob.length > 0) {
        printf("NotificationDatabase: Binding image blob (%lu bytes)\n", (unsigned long)imageBlob.length);
        sqlite3_bind_blob(statement, 8, [imageBlob bytes], (int)[imageBlob length], SQLITE_TRANSIENT);
    } else {
        printf("NotificationDatabase: No image blob to bind\n");
        sqlite3_bind_null(statement, 8);
    }

    printf("NotificationDatabase: Executing INSERT statement...\n");
    int result = sqlite3_step(statement);
    sqlite3_finalize(statement);

    if (result != SQLITE_DONE) {
        printf("NotificationDatabase ERROR: Insert failed with result code %d: %s\n", result, sqlite3_errmsg(_database));
        NSLog(@"Error inserting notification: %s", sqlite3_errmsg(_database));
        return NO;
    }

    printf("NotificationDatabase SUCCESS: Notification inserted successfully (id=%s, hasImage=%d)\n",
           [notificationId UTF8String], (imageBlob != nil && imageBlob.length > 0));
    NSLog(@"Notification inserted: id=%@, hasImage=%d", notificationId, (imageBlob != nil && imageBlob.length > 0));
    return YES;
}

- (BOOL)updateNotificationWithId:(NSString *)notificationId updates:(NSDictionary *)updates {
    if (!_database || !updates || updates.count == 0) return NO;

    // Build dynamic UPDATE query
    NSMutableString *sql = [NSMutableString stringWithString:@"UPDATE notifications SET "];
    NSMutableArray *values = [NSMutableArray array];

    BOOL first = YES;
    for (NSString *key in updates.allKeys) {
        if (!first) [sql appendString:@", "];
        first = NO;

        if ([key isEqualToString:@"title"] || [key isEqualToString:@"body"] || [key isEqualToString:@"data"]) {
            [sql appendFormat:@"%@ = ?", key];
            [values addObject:updates[key]];
        } else if ([key isEqualToString:@"trigger_timestamp"] || [key isEqualToString:@"is_scheduled"]) {
            [sql appendFormat:@"%@ = ?", key];
            [values addObject:updates[key]];
        }
    }

    [sql appendString:@" WHERE id = ?;"];

    sqlite3_stmt *statement;
    if (sqlite3_prepare_v2(_database, [sql UTF8String], -1, &statement, NULL) != SQLITE_OK) {
        NSLog(@"Error preparing update statement: %s", sqlite3_errmsg(_database));
        return NO;
    }

    int bindIndex = 1;
    for (id value in values) {
        if ([value isKindOfClass:[NSString class]]) {
            sqlite3_bind_text(statement, bindIndex, [(NSString *)value UTF8String], -1, SQLITE_TRANSIENT);
        } else if ([value isKindOfClass:[NSNumber class]]) {
            NSNumber *num = (NSNumber *)value;
            if (strcmp([num objCType], @encode(long long)) == 0) {
                sqlite3_bind_int64(statement, bindIndex, [num longLongValue]);
            } else {
                sqlite3_bind_int(statement, bindIndex, [num intValue]);
            }
        }
        bindIndex++;
    }

    sqlite3_bind_text(statement, bindIndex, [notificationId UTF8String], -1, SQLITE_TRANSIENT);

    int result = sqlite3_step(statement);
    sqlite3_finalize(statement);

    if (result != SQLITE_DONE) {
        NSLog(@"Error updating notification: %s", sqlite3_errmsg(_database));
        return NO;
    }

    NSLog(@"Notification updated: id=%@", notificationId);
    return YES;
}

- (BOOL)deleteNotificationWithId:(NSString *)notificationId {
    if (!_database) return NO;

    const char *sql = "DELETE FROM notifications WHERE id = ?;";

    sqlite3_stmt *statement;
    if (sqlite3_prepare_v2(_database, sql, -1, &statement, NULL) != SQLITE_OK) {
        NSLog(@"Error preparing delete statement: %s", sqlite3_errmsg(_database));
        return NO;
    }

    sqlite3_bind_text(statement, 1, [notificationId UTF8String], -1, SQLITE_TRANSIENT);

    int result = sqlite3_step(statement);
    sqlite3_finalize(statement);

    if (result != SQLITE_DONE) {
        NSLog(@"Error deleting notification: %s", sqlite3_errmsg(_database));
        return NO;
    }

    NSLog(@"Notification deleted: id=%@", notificationId);
    return YES;
}

- (NSArray<NSDictionary *> *)queryNotificationsWithWhere:(NSString *)whereClause
                                                 orderBy:(NSString *)orderBy
                                                   limit:(int)limit {
    if (!_database) return @[];

    // Explicitly select columns excluding image_blob for performance
    NSMutableString *sql = [NSMutableString stringWithString:@"SELECT id, title, body, trigger_timestamp, created_timestamp, is_scheduled, data FROM notifications"];

    if (whereClause && whereClause.length > 0) {
        [sql appendFormat:@" WHERE %@", whereClause];
    }

    if (orderBy && orderBy.length > 0) {
        [sql appendFormat:@" ORDER BY %@", orderBy];
    }

    if (limit > 0) {
        [sql appendFormat:@" LIMIT %d", limit];
    }

    [sql appendString:@";"];

    sqlite3_stmt *statement;
    if (sqlite3_prepare_v2(_database, [sql UTF8String], -1, &statement, NULL) != SQLITE_OK) {
        NSLog(@"Error preparing query statement: %s", sqlite3_errmsg(_database));
        return @[];
    }

    NSMutableArray *results = [NSMutableArray array];

    while (sqlite3_step(statement) == SQLITE_ROW) {
        NSMutableDictionary *row = [NSMutableDictionary dictionary];

        const char *id = (const char *)sqlite3_column_text(statement, 0);
        const char *title = (const char *)sqlite3_column_text(statement, 1);
        const char *body = (const char *)sqlite3_column_text(statement, 2);
        long long triggerTimestamp = sqlite3_column_int64(statement, 3);
        long long createdTimestamp = sqlite3_column_int64(statement, 4);
        int isScheduled = sqlite3_column_int(statement, 5);

        row[@"id"] = id ? [NSString stringWithUTF8String:id] : @"";
        row[@"title"] = title ? [NSString stringWithUTF8String:title] : @"";
        row[@"body"] = body ? [NSString stringWithUTF8String:body] : @"";
        row[@"trigger_timestamp"] = @(triggerTimestamp);
        row[@"created_timestamp"] = @(createdTimestamp);
        row[@"is_scheduled"] = @(isScheduled);

        if (sqlite3_column_type(statement, 6) != SQLITE_NULL) {
            const char *data = (const char *)sqlite3_column_text(statement, 6);
            if (data) {
                row[@"data"] = [NSString stringWithUTF8String:data];
            }
        }

        [results addObject:row];
    }

    sqlite3_finalize(statement);

    NSLog(@"Query completed: found %lu notifications", (unsigned long)results.count);
    return results;
}

- (int)countNotificationsWithWhere:(NSString *)whereClause {
    if (!_database) return 0;

    NSMutableString *sql = [NSMutableString stringWithString:@"SELECT COUNT(*) FROM notifications"];

    if (whereClause && whereClause.length > 0) {
        [sql appendFormat:@" WHERE %@", whereClause];
    }

    [sql appendString:@";"];

    sqlite3_stmt *statement;
    if (sqlite3_prepare_v2(_database, [sql UTF8String], -1, &statement, NULL) != SQLITE_OK) {
        NSLog(@"Error preparing count statement: %s", sqlite3_errmsg(_database));
        return 0;
    }

    int count = 0;
    if (sqlite3_step(statement) == SQLITE_ROW) {
        count = sqlite3_column_int(statement, 0);
    }

    sqlite3_finalize(statement);
    return count;
}

- (int)clearExpiredWithTimestamp:(long long)currentTimestamp {
    if (!_database) return 0;

    const char *sql = "DELETE FROM notifications WHERE trigger_timestamp < ?;";

    sqlite3_stmt *statement;
    if (sqlite3_prepare_v2(_database, sql, -1, &statement, NULL) != SQLITE_OK) {
        NSLog(@"Error preparing clear expired statement: %s", sqlite3_errmsg(_database));
        return 0;
    }

    sqlite3_bind_int64(statement, 1, currentTimestamp);

    int result = sqlite3_step(statement);
    int rowsDeleted = (result == SQLITE_DONE) ? (int)sqlite3_changes(_database) : 0;
    sqlite3_finalize(statement);

    NSLog(@"Expired notifications cleared: %d", rowsDeleted);
    return rowsDeleted;
}

- (BOOL)markScheduledWithId:(NSString *)notificationId isScheduled:(BOOL)isScheduled {
    if (!_database) return NO;

    const char *sql = "UPDATE notifications SET is_scheduled = ? WHERE id = ?;";

    sqlite3_stmt *statement;
    if (sqlite3_prepare_v2(_database, sql, -1, &statement, NULL) != SQLITE_OK) {
        NSLog(@"Error preparing mark scheduled statement: %s", sqlite3_errmsg(_database));
        return NO;
    }

    sqlite3_bind_int(statement, 1, isScheduled ? 1 : 0);
    sqlite3_bind_text(statement, 2, [notificationId UTF8String], -1, SQLITE_TRANSIENT);

    int result = sqlite3_step(statement);
    sqlite3_finalize(statement);

    if (result != SQLITE_DONE) {
        NSLog(@"Error marking notification scheduled: %s", sqlite3_errmsg(_database));
        return NO;
    }

    NSLog(@"Notification marked scheduled=%d: id=%@", isScheduled ? 1 : 0, notificationId);
    return YES;
}

- (NSDictionary *)getNotificationWithId:(NSString *)notificationId {
    if (!_database) return @{};

    const char *sql = "SELECT * FROM notifications WHERE id = ? LIMIT 1;";

    sqlite3_stmt *statement;
    if (sqlite3_prepare_v2(_database, sql, -1, &statement, NULL) != SQLITE_OK) {
        NSLog(@"Error preparing get notification statement: %s", sqlite3_errmsg(_database));
        return @{};
    }

    sqlite3_bind_text(statement, 1, [notificationId UTF8String], -1, SQLITE_TRANSIENT);

    NSDictionary *result = @{};

    if (sqlite3_step(statement) == SQLITE_ROW) {
        NSMutableDictionary *row = [NSMutableDictionary dictionary];

        const char *id = (const char *)sqlite3_column_text(statement, 0);
        const char *title = (const char *)sqlite3_column_text(statement, 1);
        const char *body = (const char *)sqlite3_column_text(statement, 2);
        long long triggerTimestamp = sqlite3_column_int64(statement, 3);
        long long createdTimestamp = sqlite3_column_int64(statement, 4);
        int isScheduled = sqlite3_column_int(statement, 5);

        row[@"id"] = id ? [NSString stringWithUTF8String:id] : @"";
        row[@"title"] = title ? [NSString stringWithUTF8String:title] : @"";
        row[@"body"] = body ? [NSString stringWithUTF8String:body] : @"";
        row[@"trigger_timestamp"] = @(triggerTimestamp);
        row[@"created_timestamp"] = @(createdTimestamp);
        row[@"is_scheduled"] = @(isScheduled);

        if (sqlite3_column_type(statement, 6) != SQLITE_NULL) {
            const char *data = (const char *)sqlite3_column_text(statement, 6);
            if (data) {
                row[@"data"] = [NSString stringWithUTF8String:data];
            }
        }

        result = row;
    }

    sqlite3_finalize(statement);
    return result;
}

- (int)clearAll {
    if (!_database) return 0;

    const char *sql = "DELETE FROM notifications;";

    char *errorMsg;
    int result = sqlite3_exec(_database, sql, NULL, NULL, &errorMsg);

    int rowsDeleted = 0;
    if (result != SQLITE_OK) {
        NSLog(@"Error clearing all notifications: %s", errorMsg);
        sqlite3_free(errorMsg);
    } else {
        rowsDeleted = (int)sqlite3_changes(_database);
        NSLog(@"All notifications cleared: %d", rowsDeleted);
    }

    return rowsDeleted;
}

- (NSString *)getNotificationDeepLinkWithId:(NSString *)notificationId {
    if (!_database || !notificationId) return nil;

    const char *sql = "SELECT data FROM notifications WHERE id = ? LIMIT 1;";

    sqlite3_stmt *statement;
    if (sqlite3_prepare_v2(_database, sql, -1, &statement, NULL) != SQLITE_OK) {
        NSLog(@"Error preparing deep link query: %s", sqlite3_errmsg(_database));
        return nil;
    }

    sqlite3_bind_text(statement, 1, [notificationId UTF8String], -1, SQLITE_TRANSIENT);

    NSString *deepLink = nil;
    if (sqlite3_step(statement) == SQLITE_ROW) {
        if (sqlite3_column_type(statement, 0) != SQLITE_NULL) {
            const char *data = (const char *)sqlite3_column_text(statement, 0);
            if (data) {
                deepLink = [NSString stringWithUTF8String:data];
            }
        }
    }

    sqlite3_finalize(statement);
    return deepLink;
}

- (NSData *)getNotificationImageBlobWithId:(NSString *)notificationId {
    if (!_database || !notificationId) return nil;

    const char *sql = "SELECT image_blob FROM notifications WHERE id = ? LIMIT 1;";

    sqlite3_stmt *statement;
    if (sqlite3_prepare_v2(_database, sql, -1, &statement, NULL) != SQLITE_OK) {
        NSLog(@"Error preparing image blob query: %s", sqlite3_errmsg(_database));
        return nil;
    }

    sqlite3_bind_text(statement, 1, [notificationId UTF8String], -1, SQLITE_TRANSIENT);

    NSData *imageBlob = nil;
    if (sqlite3_step(statement) == SQLITE_ROW) {
        if (sqlite3_column_type(statement, 0) != SQLITE_NULL) {
            const void *blobData = sqlite3_column_blob(statement, 0);
            int blobSize = sqlite3_column_bytes(statement, 0);
            if (blobData && blobSize > 0) {
                imageBlob = [NSData dataWithBytes:blobData length:blobSize];
            }
        }
    }

    sqlite3_finalize(statement);

    return imageBlob;
}

@end
