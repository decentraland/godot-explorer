package org.decentraland.godotexplorer

import android.content.Context
import android.util.Log
import com.tenjin.android.AttributionInfoCallback
import com.tenjin.android.TenjinSDK
import org.godotengine.godot.Dictionary

/**
 * Tenjin MMP bridge. Only started once the user accepted the EULA (the engine decides), and
 * every call is a no-op before that.
 */
class TenjinBridge {
    private val tag = "TenjinBridge"

    private var sdk: TenjinSDK? = null

    // The SDK calls back from its own thread; the result is read from the Godot thread.
    @Volatile private var attribution: Dictionary? = null
    private var attributionRequested = false

    fun start(context: Context, sdkKey: String, userId: String): Boolean {
        if (sdk != null) return true
        return try {
            val instance = TenjinSDK.getInstance(context, sdkKey)
            instance.setAppStore(TenjinSDK.AppStoreType.googleplay)
            if (userId.isNotEmpty()) instance.setCustomerUserId(userId)
            instance.connect()
            sdk = instance
            true
        } catch (e: Throwable) {
            Log.w(tag, "start failed: ${e.message}")
            false
        }
    }

    /** The SDK counts a session on every resume, not only on the first launch. */
    fun onResume() {
        try {
            sdk?.connect()
        } catch (e: Throwable) {
            Log.w(tag, "connect on resume failed: ${e.message}")
        }
    }

    fun event(name: String, value: Int): Boolean {
        val instance = sdk ?: return false
        return try {
            if (value == 0) instance.eventWithName(name) else instance.eventWithNameAndValue(name, value)
            true
        } catch (e: Throwable) {
            Log.w(tag, "event '$name' failed: ${e.message}")
            false
        }
    }

    /**
     * First call starts the fetch and returns {status: "pending"}; later calls return the cached
     * result. The SDK has no failure callback, so a fetch that never answers stays pending and
     * the engine bounds its own wait.
     */
    fun getAttribution(): Dictionary {
        attribution?.let { return it }
        val instance = sdk
        val result = Dictionary()
        if (instance == null) {
            result["status"] = "error"
            result["error"] = "not_started"
            return result
        }
        if (!attributionRequested) {
            attributionRequested = true
            try {
                instance.getAttributionInfo(AttributionInfoCallback { data ->
                    val dict = Dictionary()
                    dict["status"] = "ok"
                    data?.forEach { (key, value) -> dict[key] = value ?: "" }
                    attribution = dict
                })
            } catch (e: Throwable) {
                Log.w(tag, "getAttributionInfo failed: ${e.message}")
                val dict = Dictionary()
                dict["status"] = "error"
                dict["error"] = "${e.javaClass.simpleName}: ${e.message}"
                attribution = dict
                return dict
            }
        }
        result["status"] = "pending"
        return result
    }
}
