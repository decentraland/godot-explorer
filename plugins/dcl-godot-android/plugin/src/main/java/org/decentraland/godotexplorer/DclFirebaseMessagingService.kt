package org.decentraland.godotexplorer

import android.util.Log
import com.google.firebase.messaging.FirebaseMessagingService
import com.google.firebase.messaging.RemoteMessage

/**
 * Receives remote pushes from FCM.
 *
 * The server sends **data-only** messages (no `notification` block) at HIGH priority, which is
 * what makes this run in every app state — killed, background and foreground alike. A message
 * carrying a `notification` block would instead be drawn by the system whenever the app is not
 * in the foreground and [onMessageReceived] would never be called, taking the channel choice,
 * the de-duplication and the deep link out of our hands. That is also why testing from the
 * Firebase console composer does not exercise this path: the composer always sends a
 * `notification` block.
 *
 * FCM starts this service on its own, so there is frequently no Godot process alive here.
 * Nothing in this file may assume one: state goes to [PushTokenStore], and the plugin is only
 * ever *offered* the news through a hook that no-ops when it isn't running.
 *
 * Expected data payload:
 * ```
 * v            protocol version, currently "1"
 * push_id      unique per delivery; de-duplication key
 * category     "liveops" (unknown values fall back to it rather than dropping the message)
 * title, body  text to render
 * deep_link    decentraland://... — optional
 * ```
 */
class DclFirebaseMessagingService : FirebaseMessagingService() {

    companion object {
        private const val TAG = "DclFcmService"
    }

    /**
     * Called when FCM mints a token — first run after install, and on rotation. It does NOT
     * fire on an ordinary launch, so the plugin also asks for the token explicitly at startup;
     * this only covers the case where it changes while we are not looking.
     */
    override fun onNewToken(token: String) {
        Log.i(TAG, "FCM token refreshed (len=${token.length})")
        PushTokenStore.saveToken(applicationContext, token)
        GodotAndroidPlugin.notifyFcmTokenRefreshed(token)
    }

    override fun onMessageReceived(message: RemoteMessage) {
        val data = message.data
        Log.d(TAG, "Push received: keys=${data.keys}")

        // Fall back to the FCM message id so a payload that forgot push_id is still
        // de-duplicated rather than being treated as a brand new message on every retry.
        val pushId = data["push_id"]?.takeIf { it.isNotEmpty() }
            ?: message.messageId
            ?: ""

        if (!PushTokenStore.markSeen(applicationContext, pushId)) {
            Log.d(TAG, "Duplicate push ignored: $pushId")
            return
        }

        val title = data["title"] ?: ""
        val body = data["body"] ?: ""
        if (title.isEmpty() && body.isEmpty()) {
            Log.w(TAG, "Push with no title and no body, ignoring: $pushId")
            return
        }

        // One category in v1. An unknown one still gets shown — a message the user was meant
        // to see is worse lost than shown on a slightly wrong channel.
        val category = data["category"] ?: "liveops"
        if (category != "liveops") {
            Log.w(TAG, "Unknown category '$category', falling back to liveops")
        }

        PushNotificationBuilder.show(
            context = applicationContext,
            trayId = PushNotificationBuilder.trayIdForPush(pushId),
            channelId = PushNotificationBuilder.PUSH_CHANNEL_ID,
            channelName = PushNotificationBuilder.PUSH_CHANNEL_NAME,
            channelDescription = PushNotificationBuilder.PUSH_CHANNEL_DESCRIPTION,
            title = title,
            body = body,
            imageBlob = downloadImage(data["image_url"]),
            deepLink = data["deep_link"] ?: ""
        )
    }

    /**
     * Fetch the notification image, or return null to render text-only.
     *
     * Runs synchronously on the thread FCM already gave us, which is allowed but strictly
     * budgeted — the process can be killed once [onMessageReceived] returns, so the timeouts
     * are what keep a slow CDN from costing us the notification entirely. Any failure
     * degrades to text rather than propagating: a push without its picture still delivers the
     * message, a push that never posts does not.
     */
    private fun downloadImage(url: String?): ByteArray? {
        if (url.isNullOrEmpty()) return null
        return try {
            val connection = (java.net.URL(url).openConnection() as java.net.HttpURLConnection)
                .apply {
                    connectTimeout = IMAGE_CONNECT_TIMEOUT_MS
                    readTimeout = IMAGE_READ_TIMEOUT_MS
                    instanceFollowRedirects = true
                }
            try {
                if (connection.responseCode !in 200..299) {
                    Log.w(TAG, "Image fetch returned HTTP ${connection.responseCode}")
                    return null
                }
                connection.inputStream.use { stream -> readCapped(stream) }
            } finally {
                connection.disconnect()
            }
        } catch (e: Exception) {
            Log.w(TAG, "Image fetch failed, showing text-only: ${e.message}")
            null
        }
    }

    /**
     * Read at most [IMAGE_MAX_BYTES], returning null if the stream exceeds it.
     *
     * The cap is enforced *while* reading, not after: the URL comes off the wire, and
     * `readBytes(n)` would treat n as a buffer-size hint and happily pull a gigabyte into the
     * heap of a background service before anyone could check the size.
     */
    private fun readCapped(stream: java.io.InputStream): ByteArray? {
        val out = java.io.ByteArrayOutputStream()
        val buffer = ByteArray(16 * 1024)
        var total = 0
        while (true) {
            val read = stream.read(buffer)
            if (read < 0) break
            total += read
            if (total > IMAGE_MAX_BYTES) {
                Log.w(TAG, "Image exceeds ${IMAGE_MAX_BYTES}B, showing text-only")
                return null
            }
            out.write(buffer, 0, read)
        }
        return if (total == 0) null else out.toByteArray()
    }
}

private const val IMAGE_CONNECT_TIMEOUT_MS = 3000
private const val IMAGE_READ_TIMEOUT_MS = 4000

/** Roughly a full-width BigPicture at xxhdpi; past this the tray downscales it anyway. */
private const val IMAGE_MAX_BYTES = 1024 * 1024
