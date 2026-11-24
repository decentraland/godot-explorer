package org.decentraland.godotexplorer

import android.content.Context
import android.util.Log
import com.google.android.gms.ads.identifier.AdvertisingIdClient
import com.google.android.gms.appset.AppSet
import com.google.android.gms.tasks.Tasks
import org.godotengine.godot.Dictionary
import java.util.concurrent.Executors
import java.util.concurrent.TimeUnit

/**
 * Google Advertising ID and App Set ID. Both are blocking Play services calls, so they are read
 * on a background thread and cached; the engine polls [snapshot]. Values never go to the log.
 */
class AdvertisingIds {
    private val tag = "AdvertisingIds"
    private val executor = Executors.newSingleThreadExecutor()

    @Volatile private var result: Dictionary? = null
    @Volatile private var inFlight = false

    /** Called on start and on every resume: the player can delete the ads ID in Settings. */
    fun refresh(context: Context) {
        if (inFlight) return
        inFlight = true
        val appContext = context.applicationContext
        executor.execute {
            val dict = Dictionary()
            var gaid = ""
            var limitAdTracking = true
            try {
                val info = AdvertisingIdClient.getAdvertisingIdInfo(appContext)
                val id = info.id.orEmpty()
                // A deleted ads ID reads back as all zeros.
                if (id.isNotEmpty() && id != ZERO_ID) gaid = id
                limitAdTracking = info.isLimitAdTrackingEnabled || gaid.isEmpty()
            } catch (e: Throwable) {
                Log.w(tag, "Advertising ID unavailable: ${e.javaClass.simpleName}")
            }
            var appSetId = ""
            try {
                val info = Tasks.await(AppSet.getClient(appContext).getAppSetIdInfo(), 5, TimeUnit.SECONDS)
                appSetId = info.id.orEmpty()
            } catch (e: Throwable) {
                Log.w(tag, "App Set ID unavailable: ${e.javaClass.simpleName}")
            }
            dict["status"] = "ok"
            dict["gaid"] = gaid
            dict["limit_ad_tracking"] = limitAdTracking
            dict["app_set_id"] = appSetId
            result = dict
            inFlight = false
        }
    }

    /** The last read, or `{status: "pending"}` until the first one finishes. */
    fun snapshot(): Dictionary {
        result?.let { return it }
        val pending = Dictionary()
        pending["status"] = "pending"
        return pending
    }

    private companion object {
        const val ZERO_ID = "00000000-0000-0000-0000-000000000000"
    }
}
