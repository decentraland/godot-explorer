package com.dclrn.platform

import android.content.Context
import android.content.SharedPreferences
import android.os.Bundle
import android.util.Log
import com.android.installreferrer.api.InstallReferrerClient
import com.android.installreferrer.api.InstallReferrerStateListener
import com.google.firebase.analytics.FirebaseAnalytics
import org.decentraland.core.host.MarketingService
import org.json.JSONObject

/**
 * Firebase Analytics and the two install attribution sources (issue #2670) for the engine's
 * metrics. Called from the engine's thread; nothing here may throw into Rust. The attribution
 * readings are JSON objects with a `status`, as `dcl_core::analytics::install_attribution` reads.
 */
class AndroidMarketing(private val context: Context) : MarketingService {

  private val analytics: FirebaseAnalytics? =
      try {
        FirebaseAnalytics.getInstance(context)
      } catch (e: Throwable) {
        Log.w(TAG, "[Firebase] unavailable: ${e.message}")
        null
      }

  @Volatile private var appInstanceId = ""

  @Volatile private var installReferrer: String? = null
  @Volatile private var installReferrerStarted = false

  @Volatile private var deferredDeepLink: String? = null
  private var deferredDeepLinkStarted = false
  // SharedPreferences keeps listeners weakly: a local would be collected and never fire.
  private var deferredDeepLinkListener: SharedPreferences.OnSharedPreferenceChangeListener? = null

  init {
    try {
      analytics?.appInstanceId?.addOnCompleteListener { task ->
        if (task.isSuccessful) appInstanceId = task.result ?: ""
        else Log.w(TAG, "[Firebase] getAppInstanceId failed: ${task.exception?.message}")
      }
    } catch (e: Throwable) {
      Log.w(TAG, "[Firebase] getAppInstanceId unavailable: ${e.message}")
    }
  }

  override fun appInstanceId(): String = appInstanceId

  override fun setUserId(userId: String) {
    try {
      analytics?.setUserId(userId.ifEmpty { null })
    } catch (e: Throwable) {
      Log.w(TAG, "[Firebase] setUserId error: ${e.message}")
    }
  }

  override fun setUserProperty(name: String, value: String) {
    try {
      analytics?.setUserProperty(name, value.ifEmpty { null })
    } catch (e: Throwable) {
      Log.w(TAG, "[Firebase] setUserProperty error: ${e.message}")
    }
  }

  override fun logEvent(name: String, paramsJson: String): Boolean {
    val firebase = analytics ?: return false
    return try {
      val bundle = Bundle()
      if (paramsJson.isNotEmpty()) {
        val json = JSONObject(paramsJson)
        for (key in json.keys()) {
          when (val v = json.get(key)) {
            is String -> bundle.putString(key, v)
            is Int -> bundle.putLong(key, v.toLong())
            is Long -> bundle.putLong(key, v)
            is Double -> bundle.putDouble(key, v)
            is Boolean -> bundle.putLong(key, if (v) 1L else 0L)
            else -> bundle.putString(key, v.toString())
          }
        }
      }
      firebase.logEvent(name, bundle)
      true
    } catch (e: Throwable) {
      Log.e(TAG, "[Firebase] logEvent('$name') error: ${e.message}", e)
      false
    }
  }

  /** The first call starts the Play connection; later calls return its answer once it came. */
  override fun installReferrer(): String {
    installReferrer?.let {
      return it
    }
    if (installReferrerStarted) return PENDING
    installReferrerStarted = true
    try {
      val client = InstallReferrerClient.newBuilder(context).build()
      client.startConnection(
          object : InstallReferrerStateListener {
            override fun onInstallReferrerSetupFinished(responseCode: Int) {
              installReferrer =
                  when (responseCode) {
                    InstallReferrerClient.InstallReferrerResponse.OK ->
                        try {
                          val details = client.installReferrer
                          JSONObject()
                              .put("status", "ok")
                              .put("referrer", details.installReferrer ?: "")
                              .put("click_timestamp", details.referrerClickTimestampSeconds)
                              .put("install_timestamp", details.installBeginTimestampSeconds)
                              .put("google_play_instant", details.googlePlayInstantParam)
                              .toString()
                        } catch (e: Exception) {
                          failure("error", "Failed to read referrer: ${e.message}")
                        }
                    InstallReferrerClient.InstallReferrerResponse.FEATURE_NOT_SUPPORTED ->
                        failure("not_available", "Install referrer not supported on this device")
                    InstallReferrerClient.InstallReferrerResponse.SERVICE_UNAVAILABLE ->
                        failure("not_available", "Google Play Store service unavailable")
                    else -> failure("error", "Unknown response code: $responseCode")
                  }
              Log.i(TAG, "[IR] install referrer resolved: $installReferrer")
              try {
                client.endConnection()
              } catch (e: Exception) {
                Log.w(TAG, "[IR] Error ending install referrer connection", e)
              }
            }

            override fun onInstallReferrerServiceDisconnected() {
              if (installReferrer == null) {
                installReferrer = failure("error", "Service disconnected before data was received")
              }
            }
          })
    } catch (e: Throwable) {
      installReferrer = failure("error", "Setup ${e.javaClass.simpleName}: ${e.message}")
      return installReferrer!!
    }
    return PENDING
  }

  /**
   * GA4F writes the ad group's deep link into private SharedPreferences on the first launch after
   * an ad-driven install, racing startup: read it now, else watch for it. An organic install never
   * gets one, so this stays `pending` and the core bounds its wait.
   */
  override fun deferredDeepLink(): String {
    deferredDeepLink?.let {
      return it
    }
    return try {
      val prefs = context.getSharedPreferences(GA4F_PREFS, Context.MODE_PRIVATE)
      readDeferredDeepLink(prefs)?.let {
        deferredDeepLink = it
        return it
      }
      synchronized(this) {
        if (!deferredDeepLinkStarted) {
          deferredDeepLinkStarted = true
          val listener =
              SharedPreferences.OnSharedPreferenceChangeListener { changed, _ ->
                // Dispatched on the main thread: a throw here would take the process down.
                try {
                  if (deferredDeepLink == null) {
                    readDeferredDeepLink(changed)?.let { deferredDeepLink = it }
                  }
                } catch (e: Throwable) {
                  Log.e(TAG, "[DDL] Failed to read the deferred deep link", e)
                }
              }
          deferredDeepLinkListener = listener
          prefs.registerOnSharedPreferenceChangeListener(listener)
        }
      }
      PENDING
    } catch (e: Throwable) {
      Log.e(TAG, "[DDL] Failed to read the deferred deep link prefs", e)
      failure("error", "${e.javaClass.simpleName}: ${e.message}").also { deferredDeepLink = it }
    }
  }

  private fun readDeferredDeepLink(prefs: SharedPreferences): String? {
    val link = prefs.getString("deeplink", null)
    if (link.isNullOrEmpty()) return null
    // Stored as Double.doubleToRawLongBits(seconds), not as a number (play-services-measurement
    // 22.3.0): read as a plain Long a real click time comes back as ~4.79e15.
    val bits = prefs.getLong("timestamp", 0L)
    val clickSeconds = if (bits == 0L) 0L else Double.fromBits(bits).toLong()
    return JSONObject()
        .put("status", "ok")
        .put("deeplink", link)
        .put("click_timestamp", clickSeconds)
        .toString()
  }

  private fun failure(status: String, error: String): String =
      JSONObject().put("status", status).put("error", error).toString()

  companion object {
    private const val TAG = "AndroidMarketing"
    private const val GA4F_PREFS = "google.analytics.deferred.deeplink.prefs"
    private val PENDING = JSONObject().put("status", "pending").toString()
  }
}
