package com.dclrn

import com.facebook.react.ReactActivity
import com.facebook.react.ReactActivityDelegate
import com.facebook.react.defaults.DefaultReactActivityDelegate

/** The RN app without the Godot engine (app/godot not synced): the UI runs on the mock bridge. */
class MainActivity : ReactActivity() {

  override fun getMainComponentName(): String = "DclRn"

  override fun createReactActivityDelegate(): ReactActivityDelegate =
      DefaultReactActivityDelegate(this, mainComponentName)
}
