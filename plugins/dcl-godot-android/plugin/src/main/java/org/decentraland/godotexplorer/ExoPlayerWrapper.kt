package org.decentraland.godotexplorer

import android.content.Context
import android.graphics.ImageFormat
import android.hardware.HardwareBuffer
import android.media.ImageReader
import android.net.Uri
import android.os.Build
import android.os.Handler
import android.os.HandlerThread
import android.os.Looper
import android.util.Log
import android.view.Surface
import androidx.annotation.RequiresApi
import androidx.media3.common.AudioAttributes
import androidx.media3.common.C
import androidx.media3.common.MediaItem
import androidx.media3.common.Player
import androidx.media3.common.VideoSize
import androidx.media3.exoplayer.ExoPlayer
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicBoolean
import java.util.concurrent.atomic.AtomicLong
import java.util.concurrent.atomic.AtomicReference

/**
 * ExoPlayer wrapper for Godot integration.
 *
 * This class decodes video using hardware acceleration via ImageReader.
 *
 * GPU Mode (API 29+): Uses HardwareBuffer for zero-copy GPU texture sharing.
 *    The HardwareBuffer is passed directly to Godot's Vulkan renderer.
 *
 * Architecture:
 * - ExoPlayer runs on the main thread (Android requirement)
 * - Video frames are decoded to an ImageReader surface
 * - ImageReader callback runs on a background thread, storing the latest frame
 * - GPU mode: getHardwareBufferPtr() returns native AHardwareBuffer* for Vulkan import
 */
class ExoPlayerWrapper(private val context: Context, private val playerId: Int) {
    private val TAG = "ExoPlayerWrapper"

    // Main thread handler for ExoPlayer operations
    private val mainHandler = Handler(Looper.getMainLooper())

    // ExoPlayer instance (only accessed on main thread)
    private var player: ExoPlayer? = null

    // ImageReader for capturing decoded frames
    private var imageReader: ImageReader? = null
    private var surface: Surface? = null
    private var imageReaderThread: HandlerThread? = null
    private var imageReaderHandler: Handler? = null

    // Frame dimensions
    private var textureWidth: Int = 0
    private var textureHeight: Int = 0

    // Thread-safe state variables
    private val isPrepared = AtomicBoolean(false)
    private val isPlayingState = AtomicBoolean(false)
    private val currentPosition = AtomicLong(0)
    private val duration = AtomicLong(0)
    private val videoWidth = AtomicReference(0)
    private val videoHeight = AtomicReference(0)
    private val volume = AtomicReference(1.0f)

    // Flag to indicate video size has changed and surface needs reinitialization
    private val videoSizeChanged = AtomicBoolean(false)

    private var initError: String? = null

    @Volatile
    private var latestHardwareBuffer: HardwareBuffer? = null
    // Keep the Image alive while we're using its HardwareBuffer
    @Volatile
    private var latestHardwareBufferImage: android.media.Image? = null
    // Double buffering: keep previous frame while current is being used by GPU
    @Volatile
    private var previousHardwareBufferImage: android.media.Image? = null
    // Flag to track if the current frame has been consumed by Godot
    @Volatile
    private var hardwareBufferConsumed: Boolean = true
    private val hardwareBufferLock = Any()

    init {
        initializePlayer()
    }

    private fun initializePlayer() {
        try {
            mainHandler.post {
                try {
                    val loadControl = androidx.media3.exoplayer.DefaultLoadControl.Builder()
                        .setBufferDurationsMs(
                            /* minBufferMs = */ 5000,
                            /* maxBufferMs = */ 20000,
                            /* bufferForPlaybackMs = */ 500,
                            /* bufferForPlaybackAfterRebufferMs = */ 2000
                        )
                        .setPrioritizeTimeOverSizeThresholds(true)
                        .build()

                    val audioAttributes = AudioAttributes.Builder()
                        .setUsage(C.USAGE_MEDIA)
                        .setContentType(C.AUDIO_CONTENT_TYPE_MOVIE)
                        .build()

                    player = ExoPlayer.Builder(context)
                        .setLoadControl(loadControl)
                        .build()
                        .apply {
                            // handleAudioFocus=false allows multiple players to play simultaneously
                            setAudioAttributes(audioAttributes, /* handleAudioFocus= */ false)
                            addListener(object : Player.Listener {
                                override fun onPlaybackStateChanged(playbackState: Int) {
                                    when (playbackState) {
                                        Player.STATE_READY -> {
                                            this@ExoPlayerWrapper.isPrepared.set(true)
                                            this@ExoPlayerWrapper.updateCachedState()
                                        }
                                        Player.STATE_ENDED -> {
                                            this@ExoPlayerWrapper.isPlayingState.set(false)
                                        }
                                    }
                                }

                                override fun onIsPlayingChanged(isPlaying: Boolean) {
                                    this@ExoPlayerWrapper.isPlayingState.set(isPlaying)
                                }

                                override fun onVideoSizeChanged(size: VideoSize) {
                                    if (size.width > 0 && size.height > 0) {
                                        val previousWidth = this@ExoPlayerWrapper.videoWidth.get()
                                        val previousHeight = this@ExoPlayerWrapper.videoHeight.get()
                                        this@ExoPlayerWrapper.videoWidth.set(size.width)
                                        this@ExoPlayerWrapper.videoHeight.set(size.height)

                                        // Mark that video size changed if dimensions are different
                                        if (previousWidth != size.width || previousHeight != size.height) {
                                            Log.d(TAG, "Video size changed: ${previousWidth}x${previousHeight} -> ${size.width}x${size.height}")
                                            this@ExoPlayerWrapper.videoSizeChanged.set(true)
                                        }
                                    }
                                }

                                override fun onPlayerError(error: androidx.media3.common.PlaybackException) {
                                    Log.e(TAG, "Playback error: ${error.message}", error)
                                }
                            })

                            repeatMode = Player.REPEAT_MODE_OFF
                        }

                    this@ExoPlayerWrapper.initError = null
                } catch (e: Exception) {
                    this@ExoPlayerWrapper.initError = "Failed to initialize ExoPlayer: ${e.message}"
                    Log.e(TAG, this@ExoPlayerWrapper.initError!!, e)
                }
            }
        } catch (e: Exception) {
            initError = "Failed to schedule ExoPlayer initialization: ${e.message}"
            Log.e(TAG, initError!!, e)
        }
    }

    private fun updateCachedState() {
        player?.let { p ->
            currentPosition.set(p.currentPosition)
            duration.set(p.duration)
            videoWidth.set(p.videoSize.width)
            videoHeight.set(p.videoSize.height)
            volume.set(p.volume)
        }
    }

    private fun <T> runOnMainThreadSync(timeoutMs: Long = 1000, action: () -> T): T? {
        val result = AtomicReference<T?>()
        val latch = CountDownLatch(1)

        mainHandler.post {
            try {
                result.set(action())
            } catch (e: Exception) {
                Log.e(TAG, "Error executing action on main thread", e)
            } finally {
                latch.countDown()
            }
        }

        latch.await(timeoutMs, TimeUnit.MILLISECONDS)
        return result.get()
    }

    private fun runOnMainThreadAsync(action: () -> Unit) {
        mainHandler.post {
            try {
                action()
            } catch (e: Exception) {
                Log.e(TAG, "Error executing async action on main thread", e)
            }
        }
    }

    fun isInitialized(): Boolean = player != null && initError == null

    fun getInitError(): String? = initError

    /**
     * Initialize the video surface for frame capture.
     *
     * @param width Desired frame width
     * @param height Desired frame height
     * @return 1 on success (CPU mode), -1 on failure
     */
    fun initializeSurface(width: Int, height: Int): Int {
        return try {
            cleanupSurface()

            textureWidth = width
            textureHeight = height

            imageReaderThread = HandlerThread("ExoPlayerImageReader").apply { start() }
            imageReaderHandler = Handler(imageReaderThread!!.looper)

            // Try GPU mode if requested and API level supports it
            // GPU mode uses HardwareBuffer -> AHardwareBuffer -> Vulkan import with YCbCr conversion.
            // The YCbCr-to-RGB conversion is performed by Godot's copy_effects shader.
            val gpuResult = initializeSurfaceGpuMode(width, height)
            if (gpuResult > 0) {
                gpuResult
            } else {
                Log.e(TAG, "Failed to initialize surface")
                -1
            }
        } catch (e: Exception) {
            Log.e(TAG, "Failed to initialize surface", e)
            -1
        }
    }

    /**
     * Initialize surface in GPU mode using HardwareBuffer.
     * Requires API 29+ (Android Q).
     *
     * @return 2 on success, -1 on failure
     */
    @RequiresApi(Build.VERSION_CODES.Q)
    private fun initializeSurfaceGpuMode(width: Int, height: Int): Int {
        return try {
            // Create ImageReader with PRIVATE format and USAGE_GPU_SAMPLED_IMAGE
            // This allows direct GPU texture sharing without CPU readback
            imageReader = ImageReader.newInstance(
                width, height,
                ImageFormat.PRIVATE,
                4,  // maxImages - increased for double buffering
                HardwareBuffer.USAGE_GPU_SAMPLED_IMAGE or HardwareBuffer.USAGE_VIDEO_ENCODE
            ).apply {
                setOnImageAvailableListener({ reader ->
                    try {
                        val image = reader.acquireLatestImage()
                        if (image != null) {
                            val hwBuffer = image.hardwareBuffer
                            if (hwBuffer != null) {
                                synchronized(hardwareBufferLock) {
                                    // Close previous images to free up ImageReader slots
                                    previousHardwareBufferImage?.close()
                                    previousHardwareBufferImage = latestHardwareBufferImage
                                    // Keep the new image alive so its HardwareBuffer stays valid
                                    latestHardwareBufferImage = image
                                    latestHardwareBuffer = hwBuffer
                                    // Mark that we have a new unconsumed frame
                                    hardwareBufferConsumed = false
                                }
                            } else {
                                // No HardwareBuffer, close the image
                                image.close()
                            }
                        }
                    } catch (e: Exception) {
                        Log.e(TAG, "Error in GPU ImageReader callback: ${e.message}")
                    }
                }, imageReaderHandler)
            }

            surface = imageReader!!.surface

            val latch = CountDownLatch(1)
            mainHandler.post {
                try {
                    player?.setVideoSurface(surface)
                } finally {
                    latch.countDown()
                }
            }
            latch.await(2, TimeUnit.SECONDS)

            Log.d(TAG, "Initialized surface in GPU mode: ${width}x${height}")
            1
        } catch (e: Exception) {
            Log.e(TAG, "Failed to initialize GPU mode surface: ${e.message}", e)
            -1
        }
    }

    /**
     * Check if a new HardwareBuffer frame is available (GPU mode only).
     * Returns true only if there is an unconsumed frame ready to be processed.
     *
     * @return true if a new frame is available and hasn't been consumed yet
     */
    fun hasNewHardwareBuffer(): Boolean {
        synchronized(hardwareBufferLock) {
            return latestHardwareBuffer != null && !hardwareBufferConsumed
        }
    }

    /**
     * Get the current HardwareBuffer (GPU mode only).
     * The caller should NOT close this buffer; it will be closed when the next frame arrives.
     *
     * @return the current HardwareBuffer, or null if not in GPU mode or no frame available
     */
    @RequiresApi(Build.VERSION_CODES.Q)
    fun getHardwareBuffer(): HardwareBuffer? {
        synchronized(hardwareBufferLock) {
            return latestHardwareBuffer
        }
    }

    /**
     * Acquire the current HardwareBuffer and return its native AHardwareBuffer* pointer.
     * This is the key method for zero-copy GPU texture sharing.
     *
     * After calling this, the frame is marked as consumed and hasNewHardwareBuffer()
     * will return false until a new frame arrives.
     *
     * @return native AHardwareBuffer* pointer as a long (0 if not available)
     */
    @RequiresApi(Build.VERSION_CODES.Q)
    fun acquireHardwareBufferPtr(): Long {
        synchronized(hardwareBufferLock) {
            val buffer = latestHardwareBuffer ?: return 0L
            // Mark frame as consumed so we don't process it again
            hardwareBufferConsumed = true
            // Get the native AHardwareBuffer pointer via JNI
            return nativeGetHardwareBufferPtr(buffer)
        }
    }


    /**
     * Native method to get the AHardwareBuffer* pointer from a HardwareBuffer.
     * Implemented in the native library.
     */
    private external fun nativeGetHardwareBufferPtr(buffer: HardwareBuffer): Long

    companion object {
        init {
            try {
                System.loadLibrary("exoplayer_hwbuffer")
            } catch (e: UnsatisfiedLinkError) {
                Log.w("ExoPlayerWrapper", "Native library not loaded, GPU mode will use fallback: ${e.message}")
            }
        }
    }

    fun setSourceUrl(url: String): Boolean {
        return runOnMainThreadSync {
            try {
                isPrepared.set(false)
                val mediaItem = MediaItem.fromUri(Uri.parse(url))
                player?.apply {
                    setMediaItem(mediaItem)
                    playWhenReady = false
                    prepare()
                }
                true
            } catch (e: Exception) {
                Log.e(TAG, "Failed to set source URL", e)
                false
            }
        } ?: false
    }

    fun setSourceLocal(filePath: String): Boolean {
        return try {
            val uri = Uri.parse("file://$filePath")
            setSourceUrl(uri.toString())
        } catch (e: Exception) {
            Log.e(TAG, "Failed to set local source", e)
            false
        }
    }

    fun play() {
        runOnMainThreadAsync {
            player?.apply {
                playWhenReady = true
                play()
            }
        }
    }

    fun pause() {
        runOnMainThreadAsync {
            player?.pause()
            isPlayingState.set(false)
        }
    }

    fun stop() {
        runOnMainThreadAsync {
            player?.apply {
                stop()
                seekTo(0)
            }
            isPrepared.set(false)
            isPlayingState.set(false)
            currentPosition.set(0)
        }
    }

    fun setPosition(positionMs: Long) {
        runOnMainThreadAsync {
            player?.seekTo(positionMs)
            currentPosition.set(positionMs)
        }
    }

    fun getPosition(): Long {
        mainHandler.post {
            player?.let { currentPosition.set(it.currentPosition) }
        }
        return currentPosition.get()
    }

    fun getDuration(): Long = duration.get()

    fun isPlaying(): Boolean = isPlayingState.get()

    fun getVideoWidth(): Int = videoWidth.get()

    fun getVideoHeight(): Int = videoHeight.get()

    /**
     * Check if the video size has changed since last check and needs surface reinitialization.
     * This atomically clears the flag when returning true.
     *
     * @return true if video size has changed and surface should be reinitialized
     */
    fun hasVideoSizeChanged(): Boolean = videoSizeChanged.getAndSet(false)

    /**
     * Get current texture/surface width.
     *
     * @return current texture width in pixels, or 0 if not initialized
     */
    fun getTextureWidth(): Int = textureWidth

    /**
     * Get current texture/surface height.
     *
     * @return current texture height in pixels, or 0 if not initialized
     */
    fun getTextureHeight(): Int = textureHeight

    fun setVolume(vol: Float) {
        val clampedVolume = vol.coerceIn(0f, 1f)
        volume.set(clampedVolume)
        runOnMainThreadAsync {
            player?.volume = clampedVolume
        }
    }

    fun getVolume(): Float = volume.get()

    fun setLooping(loop: Boolean) {
        runOnMainThreadAsync {
            player?.repeatMode = if (loop) Player.REPEAT_MODE_ONE else Player.REPEAT_MODE_OFF
        }
    }

    private fun cleanupSurface() {
        try {

            // Clean up GPU mode resources
            synchronized(hardwareBufferLock) {
                // Close both images (this also invalidates their HardwareBuffers)
                previousHardwareBufferImage?.close()
                previousHardwareBufferImage = null
                latestHardwareBufferImage?.close()
                latestHardwareBufferImage = null
                // The HardwareBuffer is now invalid, just clear the reference
                latestHardwareBuffer = null
                hardwareBufferConsumed = true
            }

            imageReader?.close()
            imageReader = null

            imageReaderThread?.quitSafely()
            imageReaderThread = null
            imageReaderHandler = null

            surface?.release()
            surface = null
        } catch (e: Exception) {
            Log.e(TAG, "Error cleaning up surface", e)
        }
    }

    fun release() {
        val latch = CountDownLatch(1)
        mainHandler.post {
            try {
                player?.apply {
                    stop()
                    release()
                }
                player = null
                cleanupSurface()
            } catch (e: Exception) {
                Log.e(TAG, "Error releasing player", e)
            } finally {
                latch.countDown()
            }
        }

        latch.await(2, TimeUnit.SECONDS)
        isPrepared.set(false)
        isPlayingState.set(false)
    }

    fun getPlayerInfo(): String {
        return """
            Player ID: $playerId
            Playing: ${isPlaying()}
            Position: ${getPosition()}ms / ${getDuration()}ms
            Video: ${getVideoWidth()}x${getVideoHeight()}
            Volume: ${getVolume()}
        """.trimIndent()
    }
}
