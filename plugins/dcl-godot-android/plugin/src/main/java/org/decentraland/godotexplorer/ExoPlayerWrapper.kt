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
 * Two modes are supported:
 * 1. GPU Mode (API 29+): Uses HardwareBuffer for zero-copy GPU texture sharing.
 *    The HardwareBuffer is passed directly to Godot's Vulkan renderer.
 * 2. CPU Mode (fallback): Uses YUV_420_888 format, converts to RGBA on CPU,
 *    and provides the pixel data as a ByteArray for Godot to consume.
 *
 * Architecture:
 * - ExoPlayer runs on the main thread (Android requirement)
 * - Video frames are decoded to an ImageReader surface
 * - ImageReader callback runs on a background thread, storing the latest frame
 * - GPU mode: getHardwareBufferPtr() returns native AHardwareBuffer* for Vulkan import
 * - CPU mode: updateTexture() converts YUV->RGBA, getPixelData() returns the byte array
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

    // GPU mode: HardwareBuffer for zero-copy texture sharing
    private var useGpuMode: Boolean = false
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

    // CPU mode: Latest captured image (set by ImageReader callback, consumed by updateTexture)
    @Volatile
    private var latestImage: android.media.Image? = null
    private val imageLock = Any()

    // CPU mode: Pixel buffer for RGBA data (consumed by Godot)
    private var pixelBufferArray: ByteArray? = null

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
     * @param preferGpuMode If true, try to use GPU mode with HardwareBuffer (API 29+)
     * @return 1 on success (CPU mode), 2 on success (GPU mode), -1 on failure
     */
    fun initializeSurface(width: Int, height: Int, preferGpuMode: Boolean = true): Int {
        return try {
            cleanupSurface()

            textureWidth = width
            textureHeight = height

            imageReaderThread = HandlerThread("ExoPlayerImageReader").apply { start() }
            imageReaderHandler = Handler(imageReaderThread!!.looper)

            // Try GPU mode if requested and API level supports it
            // GPU mode uses HardwareBuffer -> AHardwareBuffer -> Vulkan import with YCbCr conversion.
            // The YCbCr-to-RGB conversion is performed by Godot's copy_effects shader.
            if (preferGpuMode && Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
                val gpuResult = initializeSurfaceGpuMode(width, height)
                if (gpuResult > 0) {
                    return gpuResult
                }
                // Fall back to CPU mode
                Log.w(TAG, "GPU mode failed, falling back to CPU mode")
            }
            Log.d(TAG, "Using CPU mode for video decoding")

            // CPU mode: Use YUV_420_888 format
            useGpuMode = false
            imageReader = ImageReader.newInstance(
                width, height,
                ImageFormat.YUV_420_888,
                3
            ).apply {
                setOnImageAvailableListener({ reader ->
                    try {
                        val image = reader.acquireLatestImage()
                        if (image != null) {
                            synchronized(imageLock) {
                                latestImage?.close()
                                latestImage = image
                            }
                        }
                    } catch (e: Exception) {
                        Log.e(TAG, "Error in ImageReader callback: ${e.message}")
                    }
                }, imageReaderHandler)
            }

            surface = imageReader!!.surface
            pixelBufferArray = ByteArray(width * height * 4)

            val latch = CountDownLatch(1)
            mainHandler.post {
                try {
                    player?.setVideoSurface(surface)
                } finally {
                    latch.countDown()
                }
            }
            latch.await(2, TimeUnit.SECONDS)

            Log.d(TAG, "Initialized surface in CPU mode: ${width}x${height}")
            1  // CPU mode success
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
            useGpuMode = true

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
            2  // GPU mode success
        } catch (e: Exception) {
            Log.e(TAG, "Failed to initialize GPU mode surface: ${e.message}", e)
            -1
        }
    }

    /**
     * Update the texture with the latest video frame.
     * Call this from Godot's render thread each frame.
     *
     * @return true if a new frame was processed, false otherwise
     */
    fun updateTexture(): Boolean {
        val image: android.media.Image?
        synchronized(imageLock) {
            image = latestImage
            latestImage = null
        }

        if (image == null) {
            return false
        }

        return try {
            convertYuvToRgba(image)
            true
        } catch (e: Exception) {
            Log.e(TAG, "Failed to process frame: ${e.message}", e)
            false
        } finally {
            image.close()
        }
    }

    /**
     * Convert YUV_420_888 image to RGBA byte array.
     * Uses integer math for better performance.
     */
    private fun convertYuvToRgba(image: android.media.Image) {
        val array = pixelBufferArray ?: return

        val yPlane = image.planes[0]
        val uPlane = image.planes[1]
        val vPlane = image.planes[2]

        val yBuffer = yPlane.buffer
        val uBuffer = uPlane.buffer
        val vBuffer = vPlane.buffer

        val yRowStride = yPlane.rowStride
        val uvRowStride = uPlane.rowStride
        val uvPixelStride = uPlane.pixelStride

        val width = image.width
        val height = image.height

        var outputIndex = 0

        for (row in 0 until height) {
            val yRowOffset = row * yRowStride
            val uvRowOffset = (row shr 1) * uvRowStride

            for (col in 0 until width) {
                val yIndex = yRowOffset + col
                val uvIndex = uvRowOffset + (col shr 1) * uvPixelStride

                val y = (yBuffer.get(yIndex).toInt() and 0xFF)
                val u = (uBuffer.get(uvIndex).toInt() and 0xFF) - 128
                val v = (vBuffer.get(uvIndex).toInt() and 0xFF) - 128

                // YUV to RGB conversion using integer math (BT.601)
                // R = Y + 1.402 * V
                // G = Y - 0.344 * U - 0.714 * V
                // B = Y + 1.772 * U
                var r = y + ((1436 * v) shr 10)
                var g = y - ((352 * u + 731 * v) shr 10)
                var b = y + ((1815 * u) shr 10)

                // Clamp to [0, 255]
                r = if (r < 0) 0 else if (r > 255) 255 else r
                g = if (g < 0) 0 else if (g > 255) 255 else g
                b = if (b < 0) 0 else if (b > 255) 255 else b

                array[outputIndex++] = r.toByte()
                array[outputIndex++] = g.toByte()
                array[outputIndex++] = b.toByte()
                array[outputIndex++] = 0xFF.toByte()  // Alpha
            }
        }
    }

    /**
     * Get the RGBA pixel data from the latest frame.
     * Call this after updateTexture() returns true.
     *
     * @return ByteArray containing RGBA pixel data, or null if unavailable
     */
    fun getPixelData(): ByteArray? = pixelBufferArray

    /**
     * Check if GPU mode is active.
     *
     * @return true if using GPU mode with HardwareBuffer, false if using CPU mode
     */
    fun isGpuMode(): Boolean = useGpuMode

    /**
     * Check if a new HardwareBuffer frame is available (GPU mode only).
     * Returns true only if there is an unconsumed frame ready to be processed.
     *
     * @return true if a new frame is available and hasn't been consumed yet
     */
    fun hasNewHardwareBuffer(): Boolean {
        if (!useGpuMode) return false
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
        if (!useGpuMode) return null
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
        if (!useGpuMode) return 0L
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
            // Clean up CPU mode resources
            synchronized(imageLock) {
                latestImage?.close()
                latestImage = null
            }

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

            pixelBufferArray = null
            useGpuMode = false
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
