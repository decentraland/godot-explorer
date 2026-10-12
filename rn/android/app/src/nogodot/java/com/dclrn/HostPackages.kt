package com.dclrn

import com.facebook.react.ReactPackage

/** Without the Godot engine no GodotBridge module is registered, so JS uses its mock bridge. */
object HostPackages {
  fun all(): List<ReactPackage> = emptyList()
}
