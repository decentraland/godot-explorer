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
import java.util.concurrent.atomic.AtomicInteger
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
      guard(-1) {
        val id = nextId.getAndIncrement()
        var player: ExoPlayerWrapper? = null
        // Built on the main thread, where ExoPlayer lives.
        if (Looper.myLooper() == Looper.getMainLooper()) {
          player = ExoPlayerWrapper(app, id)
        } else {
          val built = CountDownLatch(1)
          mainHandler.post {
            try {
              player = ExoPlayerWrapper(app, id)
            } catch (e: Exception) {
              Log.e(TAG, "Error creating ExoPlayer: ${e.message}", e)
            } finally {
              built.countDown()
            }
          }
          built.await()
        }
        val created = player ?: return@guard -1
        players[id] = created
        id
      }

  override fun initSurface(player: Int, width: Int, height: Int): Boolean =
      guard(false) { (players[player]?.initializeSurface(width, height) ?: -1) > 0 }

  override fun setSource(player: Int, source: String, local: Boolean): Boolean =
      guard(false) {
        val p = players[player] ?: return@guard false
        if (local) p.setSourceLocal(source) else p.setSourceUrl(source)
      }

  override fun play(player: Int) {
    guard(Unit) { players[player]?.play() }
  }

  override fun pause(player: Int) {
    guard(Unit) { players[player]?.pause() }
  }

  override fun stop(player: Int) {
    guard(Unit) { players[player]?.stop() }
  }

  override fun seek(player: Int, seconds: Double) {
    guard(Unit) { players[player]?.setPosition((seconds * 1000.0).toLong()) }
  }

  override fun position(player: Int): Double =
      guard(0.0) { (players[player]?.getPosition() ?: 0L) / 1000.0 }

  override fun duration(player: Int): Double =
      guard(0.0) { (players[player]?.getDuration() ?: 0L) / 1000.0 }

  override fun isPlaying(player: Int): Boolean = guard(false) { players[player]?.isPlaying() ?: false }

  override fun setVolume(player: Int, volume: Float) {
    guard(Unit) { players[player]?.setVolume(volume) }
  }

  override fun setLooping(player: Int, looping: Boolean) {
    guard(Unit) { players[player]?.setLooping(looping) }
  }

  override fun setRate(player: Int, rate: Float) {
    guard(Unit) { players[player]?.setPlaybackRate(rate) }
  }

  override fun takeSizeChanged(player: Int): Boolean =
      guard(false) { players[player]?.hasVideoSizeChanged() ?: false }

  override fun videoSize(player: Int): VideoSize =
      guard(VideoSize(0, 0)) {
        val p = players[player] ?: return@guard VideoSize(0, 0)
        VideoSize(p.getVideoWidth(), p.getVideoHeight())
      }

  override fun pollFrame(player: Int): ULong =
      guard(0uL) {
        val p = players[player] ?: return@guard 0uL
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.Q || !p.hasNewHardwareBuffer()) {
          0uL
        } else {
          p.acquireHardwareBufferPtr().toULong()
        }
      }

  override fun release(player: Int) {
    guard(Unit) { players.remove(player)?.release() }
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

  private inline fun <T> guard(fallback: T, block: () -> T): T =
      try {
        block()
      } catch (e: Throwable) {
        Log.e(TAG, "video backend call failed: ${e.message}", e)
        fallback
      }

  companion object {
    private const val TAG = "ExoVideoBackend"
    private const val RESUME_DEBOUNCE_MS = 500L
  }
}
