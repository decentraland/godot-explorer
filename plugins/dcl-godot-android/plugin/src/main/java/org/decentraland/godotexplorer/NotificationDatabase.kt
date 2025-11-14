package org.decentraland.godotexplorer

import android.content.ContentValues
import android.content.Context
import android.database.Cursor
import android.database.sqlite.SQLiteDatabase
import android.database.sqlite.SQLiteOpenHelper
import android.util.Log
import org.godotengine.godot.Dictionary

/**
 * SQLite database helper for managing local notification queue.
 * Implements the unified database API as specified in TASK.md Phase 3.
 */
class NotificationDatabase(context: Context) :
    SQLiteOpenHelper(context, DATABASE_NAME, null, DATABASE_VERSION) {

    companion object {
        private const val TAG = "NotificationDatabase"
        private const val DATABASE_NAME = "dcl_notifications.db"
        private const val DATABASE_VERSION = 1

        // Table and column names
        private const val TABLE_NOTIFICATIONS = "notifications"
        private const val COL_ID = "id"
        private const val COL_TITLE = "title"
        private const val COL_BODY = "body"
        private const val COL_TRIGGER_TIMESTAMP = "trigger_timestamp"
        private const val COL_CREATED_TIMESTAMP = "created_timestamp"
        private const val COL_IS_SCHEDULED = "is_scheduled"
        private const val COL_DATA = "data"
        private const val COL_IMAGE_BLOB = "image_blob"
    }

    override fun onCreate(db: SQLiteDatabase) {
        // Create notifications table
        val createTable = """
            CREATE TABLE $TABLE_NOTIFICATIONS (
                $COL_ID TEXT PRIMARY KEY,
                $COL_TITLE TEXT NOT NULL,
                $COL_BODY TEXT NOT NULL,
                $COL_TRIGGER_TIMESTAMP INTEGER NOT NULL,
                $COL_CREATED_TIMESTAMP INTEGER NOT NULL,
                $COL_IS_SCHEDULED INTEGER DEFAULT 0,
                $COL_DATA TEXT,
                $COL_IMAGE_BLOB BLOB
            )
        """.trimIndent()

        db.execSQL(createTable)

        // Create indexes for fast queries
        db.execSQL("CREATE INDEX idx_trigger_time ON $TABLE_NOTIFICATIONS($COL_TRIGGER_TIMESTAMP)")
        db.execSQL("CREATE INDEX idx_scheduled_time ON $TABLE_NOTIFICATIONS($COL_IS_SCHEDULED, $COL_TRIGGER_TIMESTAMP)")

        Log.d(TAG, "Database created successfully")
    }

    override fun onUpgrade(db: SQLiteDatabase, oldVersion: Int, newVersion: Int) {
        Log.d(TAG, "Upgrading database from v$oldVersion to v$newVersion")

        // Apply migrations step by step
        var currentVersion = oldVersion

        // Future migrations would go here as additional if blocks
        // Example:
        // if (currentVersion == 1 && newVersion >= 2) {
        //     // Migration from v1 to v2
        //     Log.d(TAG, "Applying migration v1 -> v2: ...")
        //     db.execSQL("ALTER TABLE $TABLE_NOTIFICATIONS ADD COLUMN new_column TEXT")
        //     currentVersion = 2
        // }

        Log.d(TAG, "Database migration completed successfully to v$newVersion")
    }

    /**
     * Insert or replace a notification in the database.
     *
     * @param id Unique notification ID
     * @param title Notification title
     * @param body Notification body
     * @param triggerTimestamp Unix timestamp (seconds) when notification should fire
     * @param isScheduled Whether notification is currently scheduled with OS (0 or 1)
     * @param data Optional JSON string for extra metadata
     * @param imageBlob Optional image data as byte array
     * @return true if successful
     */
    fun insertNotification(
        id: String,
        title: String,
        body: String,
        triggerTimestamp: Long,
        isScheduled: Int = 0,
        data: String? = null,
        imageBlob: ByteArray? = null
    ): Boolean {
        return try {
            val db = writableDatabase
            val values = ContentValues().apply {
                put(COL_ID, id)
                put(COL_TITLE, title)
                put(COL_BODY, body)
                put(COL_TRIGGER_TIMESTAMP, triggerTimestamp)
                put(COL_CREATED_TIMESTAMP, System.currentTimeMillis() / 1000)
                put(COL_IS_SCHEDULED, isScheduled)
                put(COL_DATA, data)
                put(COL_IMAGE_BLOB, imageBlob)
            }

            val result = db.insertWithOnConflict(
                TABLE_NOTIFICATIONS,
                null,
                values,
                SQLiteDatabase.CONFLICT_REPLACE
            )

            Log.d(TAG, "Notification inserted: id=$id, hasImage=${imageBlob != null}, result=$result")
            result != -1L
        } catch (e: Exception) {
            Log.e(TAG, "Error inserting notification: ${e.message}")
            false
        }
    }

    /**
     * Update notification fields.
     *
     * @param id Notification ID to update
     * @param updates Dictionary with field names and new values
     * @return true if successful
     */
    fun updateNotification(id: String, updates: Dictionary): Boolean {
        return try {
            val db = writableDatabase
            val values = ContentValues()

            // Map Godot Dictionary to ContentValues
            for (key in updates.keys) {
                val value = updates[key]
                when (key.toString()) {
                    "title" -> values.put(COL_TITLE, value.toString())
                    "body" -> values.put(COL_BODY, value.toString())
                    "trigger_timestamp" -> values.put(COL_TRIGGER_TIMESTAMP, (value as Number).toLong())
                    "is_scheduled" -> values.put(COL_IS_SCHEDULED, (value as Number).toInt())
                    "data" -> values.put(COL_DATA, value?.toString())
                }
            }

            val rowsAffected = db.update(
                TABLE_NOTIFICATIONS,
                values,
                "$COL_ID = ?",
                arrayOf(id)
            )

            Log.d(TAG, "Notification updated: id=$id, rowsAffected=$rowsAffected")
            rowsAffected > 0
        } catch (e: Exception) {
            Log.e(TAG, "Error updating notification: ${e.message}")
            false
        }
    }

    /**
     * Delete a notification by ID.
     *
     * @param id Notification ID to delete
     * @return true if successful
     */
    fun deleteNotification(id: String): Boolean {
        return try {
            val db = writableDatabase
            val rowsAffected = db.delete(
                TABLE_NOTIFICATIONS,
                "$COL_ID = ?",
                arrayOf(id)
            )

            Log.d(TAG, "Notification deleted: id=$id, rowsAffected=$rowsAffected")
            rowsAffected > 0
        } catch (e: Exception) {
            Log.e(TAG, "Error deleting notification: ${e.message}")
            false
        }
    }

    /**
     * Query notifications with SQL-like filters.
     * NOTE: Excludes image_blob column for performance - use getNotificationImageBlob() separately.
     *
     * @param whereClause SQL WHERE clause (without "WHERE" keyword), e.g. "is_scheduled = 0"
     * @param orderBy SQL ORDER BY clause (without "ORDER BY" keyword), e.g. "trigger_timestamp ASC"
     * @param limit Maximum number of results, or -1 for no limit
     * @return Array of notification dictionaries
     */
    fun queryNotifications(whereClause: String = "", orderBy: String = "", limit: Int = -1): Array<Dictionary> {
        val results = mutableListOf<Dictionary>()

        try {
            val db = readableDatabase
            val limitStr = if (limit > 0) limit.toString() else null

            // Explicitly select columns excluding image_blob for performance
            val columns = arrayOf(
                COL_ID,
                COL_TITLE,
                COL_BODY,
                COL_TRIGGER_TIMESTAMP,
                COL_CREATED_TIMESTAMP,
                COL_IS_SCHEDULED,
                COL_DATA
            )

            val cursor = db.query(
                TABLE_NOTIFICATIONS,
                columns, // Exclude image_blob
                whereClause.ifEmpty { null },
                null,
                null,
                null,
                orderBy.ifEmpty { null },
                limitStr
            )

            cursor.use {
                while (it.moveToNext()) {
                    results.add(cursorToDict(it))
                }
            }

            Log.d(TAG, "Query completed: found ${results.size} notifications")
        } catch (e: Exception) {
            Log.e(TAG, "Error querying notifications: ${e.message}")
        }

        return results.toTypedArray()
    }

    /**
     * Get count of notifications matching filter.
     *
     * @param whereClause SQL WHERE clause (without "WHERE" keyword)
     * @return Count of matching notifications
     */
    fun countNotifications(whereClause: String = ""): Int {
        return try {
            val db = readableDatabase
            val cursor = db.rawQuery(
                "SELECT COUNT(*) FROM $TABLE_NOTIFICATIONS" +
                        if (whereClause.isNotEmpty()) " WHERE $whereClause" else "",
                null
            )

            cursor.use {
                if (it.moveToFirst()) {
                    it.getInt(0)
                } else {
                    0
                }
            }
        } catch (e: Exception) {
            Log.e(TAG, "Error counting notifications: ${e.message}")
            0
        }
    }

    /**
     * Clear expired notifications (trigger_timestamp < current_time).
     *
     * @param currentTimestamp Current Unix timestamp (seconds)
     * @return Number of deleted notifications
     */
    fun clearExpired(currentTimestamp: Long): Int {
        return try {
            val db = writableDatabase
            val rowsDeleted = db.delete(
                TABLE_NOTIFICATIONS,
                "$COL_TRIGGER_TIMESTAMP < ?",
                arrayOf(currentTimestamp.toString())
            )

            Log.d(TAG, "Expired notifications cleared: $rowsDeleted")
            rowsDeleted
        } catch (e: Exception) {
            Log.e(TAG, "Error clearing expired notifications: ${e.message}")
            0
        }
    }

    /**
     * Mark notification as scheduled/unscheduled.
     *
     * @param id Notification ID
     * @param isScheduled true if scheduled with OS, false otherwise
     * @return true if successful
     */
    fun markScheduled(id: String, isScheduled: Boolean): Boolean {
        return try {
            val db = writableDatabase
            val values = ContentValues().apply {
                put(COL_IS_SCHEDULED, if (isScheduled) 1 else 0)
            }

            val rowsAffected = db.update(
                TABLE_NOTIFICATIONS,
                values,
                "$COL_ID = ?",
                arrayOf(id)
            )

            Log.d(TAG, "Notification marked scheduled=$isScheduled: id=$id")
            rowsAffected > 0
        } catch (e: Exception) {
            Log.e(TAG, "Error marking notification scheduled: ${e.message}")
            false
        }
    }

    /**
     * Get a single notification by ID.
     *
     * @param id Notification ID
     * @return Dictionary with notification data, or empty dictionary if not found
     */
    fun getNotification(id: String): Dictionary {
        return try {
            val db = readableDatabase
            val cursor = db.query(
                TABLE_NOTIFICATIONS,
                null,
                "$COL_ID = ?",
                arrayOf(id),
                null,
                null,
                null,
                "1"
            )

            cursor.use {
                if (it.moveToFirst()) {
                    cursorToDict(it)
                } else {
                    Dictionary()
                }
            }
        } catch (e: Exception) {
            Log.e(TAG, "Error getting notification: ${e.message}")
            Dictionary()
        }
    }

    /**
     * Clear all notifications from database.
     *
     * @return Number of deleted notifications
     */
    fun clearAll(): Int {
        return try {
            val db = writableDatabase
            val rowsDeleted = db.delete(TABLE_NOTIFICATIONS, null, null)
            Log.d(TAG, "All notifications cleared: $rowsDeleted")
            rowsDeleted
        } catch (e: Exception) {
            Log.e(TAG, "Error clearing all notifications: ${e.message}")
            0
        }
    }

    /**
     * Get the image blob for a specific notification.
     * This is separate from queryNotifications() to avoid loading images into memory unnecessarily.
     *
     * @param id Notification ID
     * @return ByteArray with image data, or null if no image or not found
     */
    fun getNotificationImageBlob(id: String): ByteArray? {
        return try {
            val db = readableDatabase
            val cursor = db.query(
                TABLE_NOTIFICATIONS,
                arrayOf(COL_IMAGE_BLOB),
                "$COL_ID = ?",
                arrayOf(id),
                null,
                null,
                null,
                "1"
            )

            cursor.use {
                if (it.moveToFirst()) {
                    val blobIdx = it.getColumnIndexOrThrow(COL_IMAGE_BLOB)
                    if (!it.isNull(blobIdx)) {
                        it.getBlob(blobIdx)
                    } else {
                        null
                    }
                } else {
                    null
                }
            }
        } catch (e: Exception) {
            Log.e(TAG, "Error getting notification image blob: ${e.message}")
            null
        }
    }

    /**
     * Convert a cursor row to a Godot Dictionary.
     * NOTE: Does not include image_blob - use getNotificationImageBlob() separately.
     */
    private fun cursorToDict(cursor: Cursor): Dictionary {
        val dict = Dictionary()

        dict["id"] = cursor.getString(cursor.getColumnIndexOrThrow(COL_ID))
        dict["title"] = cursor.getString(cursor.getColumnIndexOrThrow(COL_TITLE))
        dict["body"] = cursor.getString(cursor.getColumnIndexOrThrow(COL_BODY))
        dict["trigger_timestamp"] = cursor.getLong(cursor.getColumnIndexOrThrow(COL_TRIGGER_TIMESTAMP))
        dict["created_timestamp"] = cursor.getLong(cursor.getColumnIndexOrThrow(COL_CREATED_TIMESTAMP))
        dict["is_scheduled"] = cursor.getInt(cursor.getColumnIndexOrThrow(COL_IS_SCHEDULED))

        val dataIdx = cursor.getColumnIndexOrThrow(COL_DATA)
        if (!cursor.isNull(dataIdx)) {
            dict["data"] = cursor.getString(dataIdx)
        }

        return dict
    }
}
