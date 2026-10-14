package com.dclrn

import android.app.Application
import android.provider.Settings
import android.util.Log
import org.decentraland.core.host.HostListener
import org.decentraland.core.host.HostPlatform
import org.decentraland.core.host.addHostListener
import org.decentraland.core.host.installAttestation
import org.decentraland.core.host.install as installCore

/**
 * Creates the process's one core at launch, before React Native and before any engine, through
 * the uniffi Kotlin bindings of the same libdclgodot that Godot loads (D-6). The data dir is the
 * one the Godot-exported app used for `user://` (D-19).
 */
object CoreHost {
  private const val TAG = "CoreHost"

  /** The core asked for the engine before an activity could show it; the activity starts it. */
  @Volatile var engineWanted = false

  /** What the host listens to; the shell registers its own listener from JS. */
  private val listener =
      object : HostListener {
        override fun engineRequired() {
          Log.i(TAG, "engine_required")
          engineWanted = true
          MainActivity.instance?.let { it.runOnUiThread { it.startEngine() } }
        }

        override fun avatarPreviewMode(on: Boolean) {
          MainActivity.instance?.setSurfaceOnTop(on)
        }
      }

  /** The SSAID, which survives reinstalls; empty on devices with the known-bad value. */
  private fun deviceAnchor(app: Application): String {
    val ssaid = Settings.Secure.getString(app.contentResolver, Settings.Secure.ANDROID_ID) ?: ""
    return if (ssaid.equals("9774d56d682e549c", ignoreCase = true)) "" else ssaid
  }

  fun install(app: Application) {
    // JNI_OnLoad in libdclgodot is needed by livekit/webrtc, and the uniffi Kotlin bindings (JNA)
    // load the same library by name afterwards.
    System.loadLibrary("c++_shared")
    System.loadLibrary("dclgodot")
    installCore(
        app.filesDir.absolutePath,
        HostPlatform.ANDROID,
        deviceAnchor(app),
        BuildConfig.VERSION_NAME,
    )
    addHostListener(listener)
    installAttestation(PlayIntegrityAttestation(app))
    Log.i(TAG, "core installed")
  }
}
