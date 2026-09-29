package org.decentraland.godotexplorer

import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.net.Uri
import android.os.Build
import android.util.Log
import androidx.core.app.NotificationCompat

/**
 * Renders a notification into the system tray.
 *
 * Extracted from [NotificationReceiver] so the two producers share one code path:
 *   - locally scheduled reminders, posted by [NotificationReceiver] off an AlarmManager alarm;
 *   - remote pushes, posted by DclFirebaseMessagingService off an FCM data message.
 *
 * The channel is a parameter rather than a constant precisely because those two want
 * *different* channels: Android's per-channel settings are the only place a user can
 * silence event reminders without also silencing announcements (and vice versa), so
 * collapsing them onto one channel would take that choice away.
 *
 * Everything here has to work from a plain application [Context]: the FCM service has no
 * Activity, and neither does a BroadcastReceiver.
 */
object PushNotificationBuilder {

    private const val TAG = "PushNotificationBuilder"

    /** Locally scheduled reminders (events the user opted into). */
    const val LOCAL_CHANNEL_ID = "dcl_local_notifications"
    const val LOCAL_CHANNEL_NAME = "Decentraland Notifications"
    const val LOCAL_CHANNEL_DESCRIPTION = "Local notifications for Decentraland events"

    /** Server-sent announcements (the only push category in v1). */
    const val PUSH_CHANNEL_ID = "dcl_liveops"
    const val PUSH_CHANNEL_NAME = "News & Events"
    const val PUSH_CHANNEL_DESCRIPTION = "Announcements, events and updates from Decentraland"

    /**
     * Tray id for a remote push.
     *
     * `notify()` replaces any notification already showing under the same int, so pushes and
     * local reminders must not land on the same one. They draw from disjoint string spaces
     * already (`push_<uuid>` vs. event ids), which makes an accidental `hashCode()` collision
     * about as likely as any other 32-bit hash collision — this just keeps pushes in their own
     * half of the range so the ids are also *legible* as pushes in a bug report.
     */
    fun trayIdForPush(pushId: String): Int = (pushId.hashCode() and 0x3FFFFFFF) or 0x40000000

    /**
     * Create (or update) a notification channel. No-op below Android 8.
     *
     * Safe to call on every notification: `createNotificationChannel` is idempotent, and once
     * the channel exists the user's own importance/sound choices win over these values.
     */
    fun ensureChannel(
        context: Context,
        channelId: String,
        channelName: String,
        channelDescription: String
    ) {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.O) return
        try {
            val notificationManager =
                context.getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
            val channel = NotificationChannel(
                channelId,
                channelName,
                NotificationManager.IMPORTANCE_DEFAULT
            ).apply {
                description = channelDescription
                enableVibration(true)
                enableLights(true)
            }
            notificationManager.createNotificationChannel(channel)
        } catch (e: Exception) {
            Log.e(TAG, "Error creating notification channel $channelId: ${e.message}")
        }
    }

    /**
     * Post a notification to the tray.
     *
     * @param trayId    int the system keys the notification by; reusing one replaces it
     * @param deepLink  set as the launch intent's data, so tapping routes through the same
     *                  path as any other deep link. Empty means "just open the app".
     * @param imageBlob optional encoded bitmap, rendered as BigPictureStyle
     */
    fun show(
        context: Context,
        trayId: Int,
        channelId: String,
        channelName: String,
        channelDescription: String,
        title: String,
        body: String,
        imageBlob: ByteArray?,
        deepLink: String
    ) {
        val notificationManager =
            context.getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager

        ensureChannel(context, channelId, channelName, channelDescription)

        val launchIntent = context.packageManager
            .getLaunchIntentForPackage(context.packageName)
            ?.apply {
                flags = Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TOP
                putExtra(NotificationReceiver.EXTRA_NOTIFICATION_ID, trayId)
                if (deepLink.isNotEmpty()) {
                    data = Uri.parse(deepLink)
                }
            }

        // getActivity() throws on a null intent. A launcher-less package is not a case we
        // expect, but it must not take down the receiver/service we are running inside, so
        // fall back to a notification that just isn't tappable.
        val pendingIntent = if (launchIntent != null) {
            PendingIntent.getActivity(
                context,
                trayId,
                launchIntent,
                PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
            )
        } else {
            Log.w(TAG, "No launch intent for ${context.packageName}; notification won't be tappable")
            null
        }

        val customIconResId = context.resources.getIdentifier(
            "ic_notification",
            "drawable",
            context.packageName
        )
        val iconResId =
            if (customIconResId != 0) customIconResId else context.applicationInfo.icon

        val builder = NotificationCompat.Builder(context, channelId)
            .setContentTitle(title)
            .setContentText(body)
            .setSmallIcon(iconResId)
            .setAutoCancel(true)
            .setPriority(NotificationCompat.PRIORITY_DEFAULT)
        if (pendingIntent != null) {
            builder.setContentIntent(pendingIntent)
        }

        if (imageBlob != null) {
            try {
                val bitmap = BitmapFactory.decodeByteArray(imageBlob, 0, imageBlob.size)
                if (bitmap != null) {
                    builder.setLargeIcon(bitmap)
                    builder.setStyle(
                        NotificationCompat.BigPictureStyle()
                            .bigPicture(bitmap)
                            .bigLargeIcon(null as Bitmap?) // Hide large icon when expanded
                    )
                } else {
                    Log.w(TAG, "Failed to decode image blob")
                }
            } catch (e: Exception) {
                Log.e(TAG, "Error decoding image for notification: ${e.message}")
            }
        }

        notificationManager.notify(trayId, builder.build())
        Log.d(TAG, "Notification displayed: trayId=$trayId, channel=$channelId")
    }
}
