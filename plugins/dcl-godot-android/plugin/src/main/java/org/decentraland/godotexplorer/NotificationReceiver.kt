package org.decentraland.godotexplorer

import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.graphics.Bitmap
import android.os.Build
import android.util.Log
import androidx.core.app.NotificationCompat

/**
 * BroadcastReceiver for handling scheduled local notifications.
 * This receiver is triggered by AlarmManager when a notification needs to be displayed.
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
        const val DEFAULT_CHANNEL_ID = "dcl_local_notifications"
        const val DEFAULT_CHANNEL_NAME = "Decentraland Notifications"
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

        if (notificationId == -1) {
            Log.e(TAG, "Invalid notification ID")
            return
        }

        Log.d(TAG, "Showing notification: id=$notificationId, title=$title, hasImage=${imageBlob != null}")

        showNotification(context, notificationId, title, body, imageBlob)
    }

    private fun showNotification(context: Context, notificationId: Int, title: String, body: String, imageBlob: ByteArray?) {
        val notificationManager = context.getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager

        // Create notification channel for Android 8.0+
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            val channel = NotificationChannel(
                DEFAULT_CHANNEL_ID,
                DEFAULT_CHANNEL_NAME,
                NotificationManager.IMPORTANCE_DEFAULT
            ).apply {
                description = "Local notifications for Decentraland events"
                enableVibration(true)
                enableLights(true)
            }
            notificationManager.createNotificationChannel(channel)
        }

        // Create intent to launch the app when notification is tapped
        val launchIntent = context.packageManager.getLaunchIntentForPackage(context.packageName)?.apply {
            flags = Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TOP
            putExtra(EXTRA_NOTIFICATION_ID, notificationId)
        }

        val pendingIntent = PendingIntent.getActivity(
            context,
            notificationId,
            launchIntent,
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
        )

        // Get app icon resource ID
        val iconResId = context.applicationInfo.icon

        // Build notification with BigPictureStyle if image is available
        val builder = NotificationCompat.Builder(context, DEFAULT_CHANNEL_ID)
            .setContentTitle(title)
            .setContentText(body)
            .setSmallIcon(iconResId)
            .setContentIntent(pendingIntent)
            .setAutoCancel(true)
            .setPriority(NotificationCompat.PRIORITY_DEFAULT)

        // Add image if available
        if (imageBlob != null) {
            try {
                val bitmap = android.graphics.BitmapFactory.decodeByteArray(imageBlob, 0, imageBlob.size)
                if (bitmap != null) {
                    builder.setLargeIcon(bitmap)
                    builder.setStyle(
                        NotificationCompat.BigPictureStyle()
                            .bigPicture(bitmap)
                            .bigLargeIcon(null as Bitmap?) // Hide large icon when expanded
                    )
                    Log.d(TAG, "Image added to notification: ${bitmap.width}x${bitmap.height}")
                } else {
                    Log.w(TAG, "Failed to decode image blob")
                }
            } catch (e: Exception) {
                Log.e(TAG, "Error decoding image for notification: ${e.message}")
            }
        }

        val notification = builder.build()
        notificationManager.notify(notificationId, notification)
        Log.d(TAG, "Notification displayed successfully: id=$notificationId")
    }
}
