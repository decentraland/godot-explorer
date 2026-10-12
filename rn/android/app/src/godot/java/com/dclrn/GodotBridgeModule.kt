package com.dclrn

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
