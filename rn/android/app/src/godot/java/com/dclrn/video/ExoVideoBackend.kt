package com.dclrn.video

import android.app.Activity
import android.app.Application
import android.os.Build
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.util.Log
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicInteger
import java.util.concurrent.atomic.AtomicReference
import kotlin.concurrent.thread
import org.decentraland.core.host.VideoBackend
import org.decentraland.core.host.VideoSize

/**
 * The engine's scene video on Android: one [ExoPlayerWrapper] per player id. Called from the
 * engine's thread, so nothing here may throw into Rust. Players that were playing pause when the
 * app goes to the background and resume shortly after it comes back.
 */
class ExoVideoBackend(private val app: Application) : VideoBackend {
  private val players = ConcurrentHashMap<Int, ExoPlayerWrapper>()
  private val nextId = AtomicInteger(1)
  private val mainHandler = Handler(Looper.getMainLooper())
  private val failing = ConcurrentHashMap.newKeySet<Int>()

  // Main thread only.
  private val pausedInBackground = mutableSetOf<Int>()
  private var pendingResume: Runnable? = null

  init {
    app.registerActivityLifecycleCallbacks(
        object : Application.ActivityLifecycleCallbacks {
          override fun onActivityPaused(activity: Activity) = pauseForBackground()

          override fun onActivityResumed(activity: Activity) = resumeAfterBackground()

          override fun onActivityCreated(activity: Activity, state: Bundle?) {}

          override fun onActivityStarted(activity: Activity) {}

          override fun onActivityStopped(activity: Activity) {}

          override fun onActivitySaveInstanceState(activity: Activity, state: Bundle) {}

          override fun onActivityDestroyed(activity: Activity) {}
        })
  }

  override fun create(): Int =
      guard(NO_PLAYER, -1) {
        val id = nextId.getAndIncrement()
        // Built on the main thread, where ExoPlayer lives.
        if (Looper.myLooper() == Looper.getMainLooper()) {
          players[id] = ExoPlayerWrapper(app, id)
          return@guard id
        }
        // Bounded: the main thread can itself be waiting on the engine's thread (a surface change).
        // Whichever side claims `slot` first decides whether a late player is kept or released.
        val slot = AtomicReference<Any?>(null)
        val done = CountDownLatch(1)
        mainHandler.post {
          try {
            val built = ExoPlayerWrapper(app, id)
            // release() waits on the main thread, so a player the engine gave up on goes elsewhere.
            if (!slot.compareAndSet(null, built)) thread { built.release() }
          } catch (e: Exception) {
            Log.e(TAG, "Error creating ExoPlayer: ${e.message}", e)
          } finally {
            done.countDown()
          }
        }
        if (!done.await(CREATE_TIMEOUT_MS, TimeUnit.MILLISECONDS) &&
            slot.compareAndSet(null, ABANDONED)) {
          Log.e(TAG, "player $id not built within $CREATE_TIMEOUT_MS ms: the main thread is busy")
          return@guard -1
        }
        val player = slot.get() as? ExoPlayerWrapper ?: return@guard -1
        players[id] = player
        id
      }

  override fun initSurface(player: Int, width: Int, height: Int): Boolean =
      guard(player, false) { (players[player]?.initializeSurface(width, height) ?: -1) > 0 }

  override fun setSource(player: Int, source: String, local: Boolean): Boolean =
      guard(player, false) {
        val p = players[player] ?: return@guard false
        if (local) p.setSourceLocal(source) else p.setSourceUrl(source)
      }

  override fun play(player: Int) {
    guard(player, Unit) { players[player]?.play() }
  }

  override fun pause(player: Int) {
    guard(player, Unit) { players[player]?.pause() }
  }

  override fun stop(player: Int) {
    guard(player, Unit) { players[player]?.stop() }
  }

  override fun seek(player: Int, seconds: Double) {
    guard(player, Unit) { players[player]?.setPosition((seconds * 1000.0).toLong()) }
  }

  override fun position(player: Int): Double =
      guard(player, 0.0) { (players[player]?.getPosition() ?: 0L) / 1000.0 }

  override fun duration(player: Int): Double =
      guard(player, 0.0) { (players[player]?.getDuration() ?: 0L) / 1000.0 }

  override fun isPlaying(player: Int): Boolean =
      guard(player, false) { players[player]?.isPlaying() ?: false }

  override fun setVolume(player: Int, volume: Float) {
    guard(player, Unit) { players[player]?.setVolume(volume) }
  }

  override fun setLooping(player: Int, looping: Boolean) {
    guard(player, Unit) { players[player]?.setLooping(looping) }
  }

  override fun setRate(player: Int, rate: Float) {
    guard(player, Unit) { players[player]?.setPlaybackRate(rate) }
  }

  override fun takeSizeChanged(player: Int): Boolean =
      guard(player, false) { players[player]?.hasVideoSizeChanged() ?: false }

  override fun videoSize(player: Int): VideoSize =
      guard(player, VideoSize(0, 0)) {
        val p = players[player] ?: return@guard VideoSize(0, 0)
        VideoSize(p.getVideoWidth(), p.getVideoHeight())
      }

  override fun pollFrame(player: Int): ULong =
      guard(player, 0uL) {
        val p = players[player] ?: return@guard 0uL
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.Q || !p.hasNewHardwareBuffer()) {
          0uL
        } else {
          p.acquireHardwareBufferPtr().toULong()
        }
      }

  override fun release(player: Int) {
    guard(player, Unit) { players.remove(player)?.release() }
    failing.remove(player)
  }

  private fun pauseForBackground() {
    pendingResume?.let { mainHandler.removeCallbacks(it) }
    pendingResume = null
    pausedInBackground.clear()
    for ((id, player) in players) {
      if (player.isPlaying()) {
        pausedInBackground.add(id)
        player.pause()
      }
    }
  }

  private fun resumeAfterBackground() {
    pendingResume?.let { mainHandler.removeCallbacks(it) }
    if (pausedInBackground.isEmpty()) return
    // Debounced: the activity resumes before the surface is back.
    val resume = Runnable {
      for (id in pausedInBackground) players[id]?.play()
      pausedInBackground.clear()
      pendingResume = null
    }
    pendingResume = resume
    mainHandler.postDelayed(resume, RESUME_DEBOUNCE_MS)
  }

  // Several calls are polled every frame: a player that keeps throwing logs its first failure only.
  private inline fun <T> guard(player: Int, fallback: T, block: () -> T): T =
      try {
        block()
      } catch (e: Throwable) {
        if (failing.add(player)) {
          Log.e(TAG, "video backend call failed for player $player: ${e.message}", e)
        }
        fallback
      }

  companion object {
    private const val TAG = "ExoVideoBackend"
    private const val RESUME_DEBOUNCE_MS = 500L
    private const val CREATE_TIMEOUT_MS = 2000L
    /** Never a player id (they start at 1): the key `create()` logs its failures under. */
    private const val NO_PLAYER = 0
    private val ABANDONED = Any()
  }
}
