package com.dclrn

import android.app.Application
import android.provider.Settings
import android.util.Log
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import org.decentraland.core.host.HostListener
import org.decentraland.core.host.HostPlatform
import org.decentraland.core.host.addHostListener
import org.decentraland.core.host.debugFireHostEvents
import org.decentraland.core.host.install as installCore

/**
 * Creates the process's one core at launch, before React Native and before any engine, through
 * the uniffi Kotlin bindings of the same libdclgodot that Godot loads (D-6). The data dir is the
 * one the Godot-exported app used for `user://` (D-19).
 */
object CoreHost {
  private const val TAG = "CoreHost"

  /** What the host listens to; the shell registers its own listener from JS. */
  private val listener =
      object : HostListener {
        override fun engineRequired() {
          Log.i(TAG, "engine_required")
        }

        override fun avatarPreviewMode(on: Boolean) {
          Log.i(TAG, "avatar_preview_mode $on")
        }
      }

  /** The SSAID, which survives reinstalls; empty on devices with the known-bad value. */
  private fun deviceAnchor(app: Application): String {
    val ssaid = Settings.Secure.getString(app.contentResolver, Settings.Secure.ANDROID_ID) ?: ""
    return if (ssaid.equals("9774d56d682e549c", ignoreCase = true)) "" else ssaid
  }

  fun install(app: Application) {
    installCore(app.filesDir.absolutePath, HostPlatform.ANDROID, deviceAnchor(app))
    addHostListener(listener)
    Log.i(TAG, "core installed")
    if (app.applicationInfo.flags and android.content.pm.ApplicationInfo.FLAG_DEBUGGABLE != 0) {
      // Debug builds prove a Kotlin trait implementation is called from a tokio thread.
      CoroutineScope(Dispatchers.Default).launch {
        Log.i(TAG, "kotlin listener fired ${debugFireHostEvents(5u, 50u)} events")
      }
    }
  }
}
