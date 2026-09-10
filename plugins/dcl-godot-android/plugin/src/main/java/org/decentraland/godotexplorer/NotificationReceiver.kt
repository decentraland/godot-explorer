package org.decentraland.godotexplorer

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.util.Log

/**
 * BroadcastReceiver for handling scheduled local notifications.
 * This receiver is triggered by AlarmManager when a notification needs to be displayed.
 *
 * Rendering lives in [PushNotificationBuilder], which remote pushes share.
 */
class NotificationReceiver : BroadcastReceiver() {

    companion object {
        private const val TAG = "NotificationReceiver"
        const val NOTIFICATION_ACTION = "org.decentraland.godotexplorer.NOTIFICATION_ACTION"
        const val EXTRA_NOTIFICATION_ID = "notification_id"
        const val EXTRA_NOTIFICATION_STRING_ID = "notification_string_id"
        const val EXTRA_TITLE = "title"
        const val EXTRA_BODY = "body"
        const val EXTRA_IMAGE_BLOB = "image_blob"
        const val EXTRA_DEEP_LINK = "deep_link"

        // Kept as aliases so existing call sites (GodotAndroidPlugin.createNotificationChannel,
        // NotificationsManager.gd) keep compiling; the channel itself is owned by the builder.
        const val DEFAULT_CHANNEL_ID = PushNotificationBuilder.LOCAL_CHANNEL_ID
        const val DEFAULT_CHANNEL_NAME = PushNotificationBuilder.LOCAL_CHANNEL_NAME
    }

    override fun onReceive(context: Context, intent: Intent) {
        Log.d(TAG, "Notification broadcast received")

        if (intent.action != NOTIFICATION_ACTION) {
            Log.w(TAG, "Received unexpected action: ${intent.action}")
            return
        }

        val notificationId = intent.getIntExtra(EXTRA_NOTIFICATION_ID, -1)
        val title = intent.getStringExtra(EXTRA_TITLE) ?: "Notification"
        val body = intent.getStringExtra(EXTRA_BODY) ?: ""
        val imageBlob = intent.getByteArrayExtra(EXTRA_IMAGE_BLOB)
        val deepLink = intent.getStringExtra(EXTRA_DEEP_LINK) ?: ""

        if (notificationId == -1) {
            Log.e(TAG, "Invalid notification ID")
            return
        }

        Log.d(TAG, "Showing notification: id=$notificationId, title=$title, hasImage=${imageBlob != null}, deepLink=$deepLink")

        PushNotificationBuilder.show(
            context = context,
            trayId = notificationId,
            channelId = PushNotificationBuilder.LOCAL_CHANNEL_ID,
            channelName = PushNotificationBuilder.LOCAL_CHANNEL_NAME,
            channelDescription = PushNotificationBuilder.LOCAL_CHANNEL_DESCRIPTION,
            title = title,
            body = body,
            imageBlob = imageBlob,
            deepLink = deepLink
        )
    }
}
