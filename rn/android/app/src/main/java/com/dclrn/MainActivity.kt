package com.dclrn

import android.os.Bundle
import android.view.ViewGroup
import android.widget.FrameLayout
import com.facebook.react.ReactActivity
import com.facebook.react.ReactActivityDelegate
import com.facebook.react.defaults.DefaultNewArchitectureEntryPoint.fabricEnabled
import com.facebook.react.defaults.DefaultReactActivityDelegate
import org.godotengine.godot.Godot
import org.godotengine.godot.GodotFragment
import org.godotengine.godot.GodotHost
import org.godotengine.godot.plugin.GodotPlugin

/**
 * VIBE-CODED SPIKE: React Native activity that hosts the Godot client as a fragment *under*
 * the React root view, so the RN UI draws over the 3D world.
 */
class MainActivity : ReactActivity(), GodotHost {

  private var godotFragment: GodotFragment? = null

  override fun getMainComponentName(): String = "DclRn"

  override fun createReactActivityDelegate(): ReactActivityDelegate =
      DefaultReactActivityDelegate(this, mainComponentName, fabricEnabled)

  override fun onCreate(savedInstanceState: Bundle?) {
    super.onCreate(null)
    val content = findViewById<ViewGroup>(android.R.id.content)
    val container = FrameLayout(this).apply { id = GODOT_CONTAINER_ID }
    content.addView(
        container,
        0,
        FrameLayout.LayoutParams(
            ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.MATCH_PARENT))
    val fragment = GodotFragment()
    godotFragment = fragment
    supportFragmentManager
        .beginTransaction()
        .replace(GODOT_CONTAINER_ID, fragment)
        .commitNowAllowingStateLoss()
  }

  override fun getActivity() = this

  override fun getGodot(): Godot? = godotFragment?.godot

  override fun getCommandLine(): List<String> = BOOT_ARGS

  override fun getHostPlugins(engine: Godot): Set<GodotPlugin> = setOf(RnHostPlugin(engine))

  companion object {
    private const val GODOT_CONTAINER_ID = 0x00D0D0
    private const val MAIN_REALM = "https://realm-provider-ea.decentraland.org/main"

    // Ephemeral guest, straight into Genesis Plaza, Godot UI hidden (see rn_bridge.gd).
    private val BOOT_ARGS =
        listOf(
            "--rn-host",
            "--skip-lobby",
            "--guest-profile",
            "--force-mobile",
            "--realm",
            MAIN_REALM,
            "--location",
            "0,0")

    init {
      // JNI_OnLoad in libdclgodot is needed by livekit/webrtc before the engine starts.
      System.loadLibrary("c++_shared")
      System.loadLibrary("dclgodot")
    }
  }
}
