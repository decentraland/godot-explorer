package com.dclrn

import org.godotengine.godot.Godot
import org.godotengine.godot.plugin.GodotPlugin
import org.godotengine.godot.plugin.SignalInfo
import org.godotengine.godot.plugin.UsedByGodot

/** Godot-side end of the bridge: `rn_command` signal in, `rn_event(json)` call out. */
class RnHostPlugin(godot: Godot) : GodotPlugin(godot) {

  init {
    instance = this
  }

  override fun getPluginName() = "RnHostPlugin"

  override fun getPluginSignals(): MutableSet<SignalInfo> =
      mutableSetOf(SignalInfo("rn_command", String::class.java))

  @UsedByGodot
  fun rn_event(json: String) {
    GodotBridgeModule.dispatch(json)
  }

  fun sendCommand(json: String) {
    emitSignal("rn_command", json)
  }

  companion object {
    @Volatile var instance: RnHostPlugin? = null
  }
}
