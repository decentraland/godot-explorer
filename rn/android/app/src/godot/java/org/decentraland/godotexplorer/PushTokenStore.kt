package org.decentraland.godotexplorer

import android.content.Context
import android.util.Log

/**
 * Small persistent store for the push subsystem, backed by SharedPreferences.
 *
 * Both readers can run with neither React Native nor the engine alive — [DclFirebaseMessagingService]
 * is started by FCM on its own — so nothing here may assume them.
 */
object PushTokenStore {

    private const val TAG = "PushTokenStore"
    private const val PREFS_NAME = "dcl_push"
    private const val KEY_TOKEN = "fcm_token"
    private const val KEY_SEEN_IDS = "seen_push_ids"

    /**
     * How many delivered push ids to remember for de-duplication. FCM can deliver the same
     * message more than once, and a redelivery after the tray entry was dismissed would
     * otherwise post it again. Small on purpose: this only has to cover retries, not history.
     */
    private const val SEEN_IDS_CAPACITY = 64

    private fun prefs(context: Context) =
        context.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)

    /** Last token FCM handed us, or "" if none has arrived yet. */
    fun getToken(context: Context): String =
        try {
            prefs(context).getString(KEY_TOKEN, "") ?: ""
        } catch (e: Exception) {
            Log.e(TAG, "Error reading token: ${e.message}")
            ""
        }

    /** @return true if this differs from the token already stored. */
    fun saveToken(context: Context, token: String): Boolean =
        try {
            val p = prefs(context)
            val changed = p.getString(KEY_TOKEN, "") != token
            if (changed) {
                p.edit().putString(KEY_TOKEN, token).apply()
            }
            changed
        } catch (e: Exception) {
            Log.e(TAG, "Error saving token: ${e.message}")
            false
        }

    /**
     * Record a push id as delivered.
     *
     * @return true the first time an id is seen, false if it was already delivered — so the
     *         caller can simply `if (!markSeen(...)) return`.
     */
    @Synchronized
    fun markSeen(context: Context, pushId: String): Boolean {
        if (pushId.isEmpty()) return true
        return try {
            val p = prefs(context)
            val current = (p.getString(KEY_SEEN_IDS, "") ?: "")
                .split(',')
                .filter { it.isNotEmpty() }
            if (current.contains(pushId)) return false
            val updated = (current + pushId).takeLast(SEEN_IDS_CAPACITY)
            p.edit().putString(KEY_SEEN_IDS, updated.joinToString(",")).apply()
            true
        } catch (e: Exception) {
            // Better a duplicate notification than a dropped one.
            Log.e(TAG, "Error in de-duplication, treating as new: ${e.message}")
            true
        }
    }
}
