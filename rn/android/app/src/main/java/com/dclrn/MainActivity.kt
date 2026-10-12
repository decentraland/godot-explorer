package com.dclrn

import android.os.Bundle
import android.util.Log
import android.view.MotionEvent
import android.view.View
import android.view.ViewGroup
import android.widget.FrameLayout
import com.facebook.react.ReactActivity
import com.facebook.react.ReactActivityDelegate
import com.facebook.react.defaults.DefaultNewArchitectureEntryPoint.fabricEnabled
import com.facebook.react.defaults.DefaultReactActivityDelegate
import com.facebook.react.uimanager.TouchTargetHelper
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
  private var gestureGoesToGodot = false

  override fun getMainComponentName(): String = "DclRn"

  override fun createReactActivityDelegate(): ReactActivityDelegate =
      DefaultReactActivityDelegate(this, mainComponentName, fabricEnabled)

  override fun onCreate(savedInstanceState: Bundle?) {
    super.onCreate(null)
    instance = this
  }

  override fun onDestroy() {
    if (instance === this) instance = null
    super.onDestroy()
  }

  /** Starts the Godot engine once, under the RN root. The engine cannot be restarted in-process. */
  fun startEngine() {
    if (godotFragment != null) return
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

  // RN's root view consumes every touch, even over transparent areas, so Godot underneath
  // (joystick, gamepad, camera look) would never see one. Route each gesture by its first
  // pointer: RN when a real RN view is under it, Godot otherwise. Later pointers follow it.
  override fun dispatchTouchEvent(ev: MotionEvent): Boolean {
    val godotContainer = findViewById<View>(GODOT_CONTAINER_ID)
    val rnRoot = (godotContainer?.parent as? ViewGroup)?.let { it.getChildAt(it.childCount - 1) }
    if (godotContainer == null || rnRoot !is ViewGroup || rnRoot === godotContainer) {
      return super.dispatchTouchEvent(ev)
    }
    if (ev.actionMasked == MotionEvent.ACTION_DOWN) {
      val target = TouchTargetHelper.findTargetTagForTouch(ev.x, ev.y, rnRoot)
      gestureGoesToGodot = target == rnRoot.id || target <= 0
      Log.d(TAG, "touch down target=$target rootId=${rnRoot.id} -> godot=$gestureGoesToGodot")
    }
    return if (gestureGoesToGodot) godotContainer.dispatchTouchEvent(ev)
    else super.dispatchTouchEvent(ev)
  }

  override fun getActivity() = this

  override fun getGodot(): Godot? = godotFragment?.godot

  override fun getCommandLine(): List<String> = BOOT_ARGS

  override fun getHostPlugins(engine: Godot): Set<GodotPlugin> = setOf(RnHostPlugin(engine))

  companion object {
    private const val TAG = "DclRn"
    private const val GODOT_CONTAINER_ID = 0x00D0D0

    // No world: the engine only renders the avatar preview (see rn_avatar_host.gd).
    private val BOOT_ARGS = listOf("--rn-host", "--rn-avatar-only", "--force-mobile", "--no-pulse")

    @Volatile var instance: MainActivity? = null

    init {
      // JNI_OnLoad in libdclgodot is needed by livekit/webrtc before the engine starts.
      System.loadLibrary("c++_shared")
      System.loadLibrary("dclgodot")
    }
  }
}
