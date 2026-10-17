package com.dclrn

import com.facebook.react.ReactPackage

/** React packages that exist only when the Godot engine is linked. */
object HostPackages {
  fun all(): List<ReactPackage> = listOf(GodotBridgePackage())
}
