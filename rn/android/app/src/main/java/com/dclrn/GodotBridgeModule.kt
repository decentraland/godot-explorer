package com.dclrn

import android.content.pm.ActivityInfo
import com.facebook.react.ReactPackage
import com.facebook.react.bridge.ReactApplicationContext
import com.facebook.react.bridge.ReactContextBaseJavaModule
import com.facebook.react.bridge.ReactMethod
import com.facebook.react.modules.core.DeviceEventManagerModule
import com.facebook.react.uimanager.ViewManager

/** JS-facing end of the bridge. Commands and events are JSON strings. */
class GodotBridgeModule(context: ReactApplicationContext) : ReactContextBaseJavaModule(context) {

  init {
    reactContext = context
  }

  override fun getName() = "GodotBridge"

  @ReactMethod
  fun sendCommand(json: String) {
    RnHostPlugin.instance?.sendCommand(json)
  }

  @ReactMethod
  fun startEngine() {
    MainActivity.instance?.let { it.runOnUiThread { it.startEngine() } }
  }

  @ReactMethod
  fun setOrientation(mode: String) {
    val orientation =
        when (mode) {
          "portrait" -> ActivityInfo.SCREEN_ORIENTATION_PORTRAIT
          "landscape" -> ActivityInfo.SCREEN_ORIENTATION_SENSOR_LANDSCAPE
          else -> return
        }
    MainActivity.instance?.let { it.runOnUiThread { it.requestedOrientation = orientation } }
  }

  // Required by NativeEventEmitter.
  @ReactMethod fun addListener(eventName: String) {}

  @ReactMethod fun removeListeners(count: Int) {}

  companion object {
    @Volatile private var reactContext: ReactApplicationContext? = null

    fun dispatch(json: String) {
      reactContext
          ?.getJSModule(DeviceEventManagerModule.RCTDeviceEventEmitter::class.java)
          ?.emit("GodotEvent", json)
    }
  }
}

class GodotBridgePackage : ReactPackage {
  override fun createNativeModules(context: ReactApplicationContext) =
      listOf(GodotBridgeModule(context))

  override fun createViewManagers(context: ReactApplicationContext): List<ViewManager<*, *>> =
      emptyList()
}
