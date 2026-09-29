package org.decentraland.godotexplorer

import android.app.Activity
import android.util.Log

/**
 * Google Play In-App Review binding (issue #2739) — the native 1-5 star card, rendered in-process
 * by Play.
 *
 * WHY THIS IS ITS OWN CLASS, and why no Play Core type may appear in a signature here or on
 * GodotAndroidPlugin: at registration `GodotPlugin.onRegisterPluginWithGodotNative` calls
 * `getDeclaredMethods()` on the plugin class, which resolves the parameter types of EVERY declared
 * method — private ones included. A single unresolvable type there throws NoClassDefFoundError on
 * the Vulkan thread and kills the app at boot, before any of our code runs. Keeping Play Core
 * inside method BODIES (and behind the catch below) means a missing or stripped dependency
 * degrades to "review unavailable" instead of a fatal crash.
 *
 * Play never reports whether the card appeared or what the user did. Over the (undocumented,
 * roughly monthly) per-user quota it renders nothing, returns no error, and still completes — so
 * the caller must treat the launch itself as the event, and must never put its own UI or a
 * call-to-action in front of it. Both stores prohibit that.
 */
class InAppReview(private val logTag: String) {

    // ReviewInfo expires, so it is cached in memory only, with a timestamp, and re-requested
    // rather than fired stale. Never persisted across sessions. Typed as Any? so the field
    // carries no Play Core type into this class's surface.
    @Volatile private var cachedReviewInfo: Any? = null
    @Volatile private var cachedReviewInfoAtMs: Long = 0L

    // ReviewInfo has no documented lifetime; short enough that we never launch a stale one, long
    // enough that the prewarm is still worth doing.
    private val reviewInfoTtlMs: Long = 5 * 60 * 1000

    private fun isCachedInfoFresh(): Boolean =
        cachedReviewInfo != null &&
            (System.currentTimeMillis() - cachedReviewInfoAtMs) < reviewInfoTtlMs

    /**
     * Fetch a ReviewInfo ahead of the trigger moment — the request has real latency, so doing it
     * at launch time would stall the moment. Safe to call repeatedly; a fresh cached value
     * short-circuits. Reports nothing: a failure just leaves the cache empty and [launch]
     * re-requests.
     */
    fun prewarm(activity: Activity) {
        if (isCachedInfoFresh()) return
        try {
            val manager = com.google.android.play.core.review.ReviewManagerFactory.create(activity)
            manager.requestReviewFlow()
                .addOnSuccessListener { info ->
                    cachedReviewInfo = info
                    cachedReviewInfoAtMs = System.currentTimeMillis()
                    Log.i(logTag, "[InAppReview] prewarm ready")
                }
                .addOnFailureListener { e ->
                    cachedReviewInfo = null
                    Log.w(logTag, "[InAppReview] prewarm failed: ${e.message}", e)
                }
        } catch (e: Throwable) {
            cachedReviewInfo = null
            Log.e(logTag, "[InAppReview] prewarm error: ${e.javaClass.name}: ${e.message}", e)
        }
    }

    /**
     * Launch the review flow, re-requesting the ReviewInfo when the cached one is missing or
     * stale. Always calls [onFinished] exactly once — with "" when the flow ran, otherwise a
     * reason. "Finished" means the FLOW completed, never that the user rated.
     */
    fun launch(activity: Activity, onFinished: (String) -> Unit) {
        try {
            val manager = com.google.android.play.core.review.ReviewManagerFactory.create(activity)
            val cached = if (isCachedInfoFresh()) cachedReviewInfo else null
            if (cached != null) {
                launchFlow(activity, manager, cached, onFinished)
                return
            }
            manager.requestReviewFlow()
                .addOnSuccessListener { info ->
                    cachedReviewInfo = info
                    cachedReviewInfoAtMs = System.currentTimeMillis()
                    launchFlow(activity, manager, info, onFinished)
                }
                .addOnFailureListener { e ->
                    Log.w(logTag, "[InAppReview] request failed: ${e.message}", e)
                    onFinished("${e.javaClass.simpleName}: ${e.message}")
                }
        } catch (e: Throwable) {
            Log.e(logTag, "[InAppReview] launch error: ${e.javaClass.name}: ${e.message}", e)
            onFinished("${e.javaClass.simpleName}: ${e.message}")
        }
    }

    // The card must be launched on the UI thread and rendered as-is: no resize, opacity, overlay
    // or programmatic dismissal, and it must stay the topmost layer.
    private fun launchFlow(
        activity: Activity,
        manager: com.google.android.play.core.review.ReviewManager,
        info: Any,
        onFinished: (String) -> Unit
    ) {
        activity.runOnUiThread {
            try {
                val reviewInfo = info as com.google.android.play.core.review.ReviewInfo
                manager.launchReviewFlow(activity, reviewInfo)
                    .addOnCompleteListener {
                        // Fires regardless of outcome, including when nothing was shown at all.
                        cachedReviewInfo = null
                        Log.i(logTag, "[InAppReview] flow completed")
                        onFinished("")
                    }
            } catch (e: Throwable) {
                cachedReviewInfo = null
                Log.e(logTag, "[InAppReview] launchReviewFlow error: ${e.message}", e)
                onFinished("${e.javaClass.simpleName}: ${e.message}")
            }
        }
    }
}
