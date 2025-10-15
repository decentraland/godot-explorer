package org.decentraland.godotexplorer

import android.app.Activity
import android.app.ActivityManager
import android.content.ActivityNotFoundException
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.content.pm.ActivityInfo
import android.content.pm.PackageManager
import android.net.Uri
import android.os.BatteryManager
import android.os.Build
import android.util.Log
import android.webkit.WebResourceRequest
import android.widget.FrameLayout
import android.widget.TextView
import android.webkit.WebView
import android.webkit.WebViewClient
import android.widget.Toast
import androidx.browser.customtabs.CustomTabsIntent
import androidx.browser.customtabs.CustomTabsService
import org.godotengine.godot.Dictionary
import org.godotengine.godot.Godot
import org.godotengine.godot.plugin.GodotPlugin
import org.godotengine.godot.plugin.UsedByGodot

class GodotAndroidPlugin(godot: Godot) : GodotPlugin(godot) {

    private var webView: WebView? = null
    private var isWebViewOpen: Boolean = false
    private var overlayLayout: FrameLayout? = null

    private val customPackageNames = arrayOf(
        "com.android.chrome",        // Google Chrome
        "org.mozilla.firefox",       // Mozilla Firefox
        "com.microsoft.emmx",        // Microsoft Edge
        "com.brave.browser",         // Brave Browser
        "com.opera.browser",         // Opera Browser
        "com.opera.mini.native",     // Opera Mini
        "com.sec.android.app.sbrowser" // Samsung Internet
    )

    override fun getPluginName() = BuildConfig.GODOT_PLUGIN_NAME

    @UsedByGodot
    fun showMessage(message: String) {
        runOnUiThread {
            Toast.makeText(activity, message, Toast.LENGTH_LONG).show()
            Log.v(pluginName, message)
        }
    }

    @UsedByGodot
    fun openCustomTabUrl(url: String) {
        runOnUiThread {
            activity?.let {
                var done = false
                for (customPackageName in customPackageNames) {
                    if (openCustomTab(it, url, customPackageName)) {
                        Log.d(pluginName, "openCustomTab: $customPackageName")
                        //openCustomTab(it, url, customPackageName)
                        done = true
                        break
                    }
                }

                if (!done) {
                    openUrl(it, url)
                    Log.d(pluginName, "No Custom Tabs available, using fallback to open URL")
                }
            } ?: Log.e(pluginName, "Activity is null, cannot open URL.")
        }
    }

    private fun openCustomTab(activity: Activity, url: String, packageName: String): Boolean {
        try {
            val builder = CustomTabsIntent.Builder()
            val customTabsIntent = builder.build()
            customTabsIntent.intent.addFlags(Intent.FLAG_ACTIVITY_NO_HISTORY)
            customTabsIntent.intent.addFlags(Intent.FLAG_ACTIVITY_CLEAR_TOP)
            customTabsIntent.intent.setPackage(packageName)
            customTabsIntent.launchUrl(activity, Uri.parse(url))
            return true
        } catch (e: Exception) {
            Log.e(pluginName, "Error opening Custom Tab for package $packageName: $e")
            return false
        }
    }

    private fun openUrl(activity: Activity, url: String) {
        val intent = Intent(Intent.ACTION_VIEW, Uri.parse(url))
        try {
            activity.startActivity(intent)
        } catch (e: Exception) {
            Log.e(pluginName, "Error opening default browser: $e")
        }
    }

    private fun handleDeepLink(activity: Activity, url: String) {
        val intent = Intent(Intent.ACTION_VIEW, Uri.parse(url))
        try {
            activity.startActivity(intent)
        } catch (e: ActivityNotFoundException) {
            Log.e(pluginName, "No application can handle deep link $url: $e")
            showMessage("No application found to handle this link")
        }
    }

    @UsedByGodot
    fun openWebView(url: String, overlayText: String?) {
        runOnUiThread {
            activity?.let {
                if (!isWebViewOpen) {
                    // Change orientation to portrait
                    it.requestedOrientation = ActivityInfo.SCREEN_ORIENTATION_PORTRAIT

                    // Create a FrameLayout to hold the WebView and TextView
                    overlayLayout = FrameLayout(it)

                    // Create a WebView and configure it to behave as much like Chrome as possible
                    webView = WebView(it).apply {
                        settings.javaScriptEnabled = true
                        settings.domStorageEnabled = true
                        settings.javaScriptCanOpenWindowsAutomatically = true
                        settings.mediaPlaybackRequiresUserGesture = false
                        settings.loadsImagesAutomatically = true
                        settings.mixedContentMode = android.webkit.WebSettings.MIXED_CONTENT_ALWAYS_ALLOW
                        settings.allowFileAccess = true
                        settings.setSupportZoom(true)
                        settings.builtInZoomControls = true
                        settings.displayZoomControls = false
                        settings.useWideViewPort = true
                        settings.loadWithOverviewMode = true
                        settings.databaseEnabled = true

                        // Allow third-party cookies (to make it similar to Chrome)
                        android.webkit.CookieManager.getInstance().setAcceptThirdPartyCookies(this, true)

                        // Set a custom WebViewClient to handle deep links, redirects, SSL, etc.
                        webViewClient = object : WebViewClient() {
                            override fun shouldOverrideUrlLoading(view: WebView?, request: WebResourceRequest?): Boolean {
                                val requestUrl = request?.url.toString()
                                if (requestUrl.startsWith("wc:")) {
                                    handleDeepLink(it, requestUrl)
                                    return true
                                }

                                if (requestUrl.startsWith("decentraland:")) {
                                    closeWebView()
                                    return true
                                }
                                return false
                            }

                            override fun onReceivedSslError(view: WebView?, handler: android.webkit.SslErrorHandler?, error: android.net.http.SslError?) {
                                Log.e(pluginName, "Ssl error")
                                handler?.cancel()
                            }

                            override fun onPageStarted(view: WebView?, url: String?, favicon: android.graphics.Bitmap?) {
                                Log.d(pluginName, "Page loading started: $url")
                                super.onPageStarted(view, url, favicon)
                            }

                            override fun onPageFinished(view: WebView?, url: String?) {
                                Log.d(pluginName, "Page loading finished: $url")
                                super.onPageFinished(view, url)
                            }
                        }

                        loadUrl(url)
                    }

                    // Add the WebView to the FrameLayout
                    overlayLayout?.addView(webView, FrameLayout.LayoutParams(
                        FrameLayout.LayoutParams.MATCH_PARENT,
                        FrameLayout.LayoutParams.MATCH_PARENT
                    ))

                    // If overlayText is not null or empty, create a TextView and add it
                    if (!overlayText.isNullOrEmpty()) {
                        val textView = TextView(it).apply {
                            text = overlayText
                            textSize = 18f
                            setPadding(16, 16, 16, 16)
                            setBackgroundColor(0x80000000.toInt()) // Semi-transparent background
                            setTextColor(0xFFFFFFFF.toInt()) // White text
                        }

                        val textViewLayoutParams = FrameLayout.LayoutParams(
                            FrameLayout.LayoutParams.WRAP_CONTENT,
                            FrameLayout.LayoutParams.WRAP_CONTENT
                        ).apply {
                            gravity = android.view.Gravity.CENTER_HORIZONTAL or android.view.Gravity.BOTTOM
                            bottomMargin = (it.resources.displayMetrics.heightPixels * 0.2).toInt() // Position 20% above bottom
                        }

                        overlayLayout?.addView(textView, textViewLayoutParams)
                    }

                    // Add the FrameLayout to the activity's layout
                    it.addContentView(
                        overlayLayout,
                        FrameLayout.LayoutParams(
                            FrameLayout.LayoutParams.MATCH_PARENT,
                            FrameLayout.LayoutParams.MATCH_PARENT
                        )
                    )

                    isWebViewOpen = true
                }
            } ?: Log.e(pluginName, "Activity is null, cannot open WebView.")
        }
    }


    @UsedByGodot
    fun closeWebView() {
        runOnUiThread {
            activity?.let {
                if (isWebViewOpen && overlayLayout != null) {
                    // Remove the overlay layout from the activity's layout
                    (overlayLayout?.parent as? FrameLayout)?.removeView(overlayLayout)
                    webView?.destroy()
                    webView = null
                    overlayLayout = null
                    isWebViewOpen = false

                    // Change orientation back to landscape
                    it.requestedOrientation = ActivityInfo.SCREEN_ORIENTATION_LANDSCAPE
                }
            } ?: Log.e(pluginName, "Activity is null, cannot close WebView.")
        }
    }

    private fun isPackageAvailable(activity: Activity, packageName: String): Boolean {
        // First, check if the package supports Custom Tabs.
        val customTabsPackages = activity.packageManager.queryIntentServices(
            Intent(CustomTabsService.ACTION_CUSTOM_TABS_CONNECTION),
            PackageManager.MATCH_ALL
        )
        if (customTabsPackages.any { resolveInfo -> resolveInfo.serviceInfo.packageName.equals(packageName, ignoreCase = true) }) {
            return true
        }

        // If the package does not support Custom Tabs, fallback to check if it is installed as a browser.
        return try {
            activity.packageManager.getPackageInfo(packageName, PackageManager.GET_ACTIVITIES)
            true
        } catch (e: PackageManager.NameNotFoundException) {
            false
        }
    }

    @UsedByGodot
    fun getMobileDeviceInfo(): Dictionary {
        val info = Dictionary()

        activity?.let { ctx ->
            try {
                // Get memory usage
                val activityManager = ctx.getSystemService(Context.ACTIVITY_SERVICE) as ActivityManager
                val memInfo = ActivityManager.MemoryInfo()
                activityManager.getMemoryInfo(memInfo)

                // Current app memory usage (approximate)
                val runtime = Runtime.getRuntime()
                val usedMemoryMB = (runtime.totalMemory() - runtime.freeMemory()) / (1024 * 1024)
                info["memory_usage"] = usedMemoryMB.toInt()

                // Total RAM
                val totalRamMB = memInfo.totalMem / (1024 * 1024)
                info["total_ram_mb"] = totalRamMB.toInt()

                // Get battery information
                val batteryManager = ctx.getSystemService(Context.BATTERY_SERVICE) as BatteryManager

                // Battery temperature (in tenths of a degree Celsius)
                val batteryIntentFilter = IntentFilter(Intent.ACTION_BATTERY_CHANGED)
                val batteryStatus = ctx.registerReceiver(null, batteryIntentFilter)
                val temperature = batteryStatus?.getIntExtra(BatteryManager.EXTRA_TEMPERATURE, -1) ?: -1
                val temperatureCelsius = if (temperature > 0) temperature / 10.0f else -1.0f

                info["device_temperature_celsius"] = temperatureCelsius

                // Approximate thermal state based on temperature
                val thermalState = when {
                    temperatureCelsius < 0 -> "unknown"
                    temperatureCelsius < 40.0f -> "nominal"
                    temperatureCelsius < 45.0f -> "fair"
                    temperatureCelsius < 50.0f -> "serious"
                    else -> "critical"
                }
                info["thermal_state"] = thermalState

                // Battery level, status, and charging state
                val batteryLevel = batteryStatus?.getIntExtra(BatteryManager.EXTRA_LEVEL, -1) ?: -1
                val batteryScale = batteryStatus?.getIntExtra(BatteryManager.EXTRA_SCALE, -1) ?: -1
                val batteryPct = if (batteryLevel >= 0 && batteryScale > 0) {
                    (batteryLevel.toFloat() / batteryScale.toFloat()) * 100.0f
                } else {
                    -1.0f
                }

                // Get charging state with detailed type information
                val plugged = batteryStatus?.getIntExtra(BatteryManager.EXTRA_PLUGGED, -1) ?: -1
                val status = batteryStatus?.getIntExtra(BatteryManager.EXTRA_STATUS, -1) ?: -1

                val chargingState = when {
                    status == BatteryManager.BATTERY_STATUS_FULL -> "full"
                    plugged == BatteryManager.BATTERY_PLUGGED_AC -> "plugged"
                    plugged == BatteryManager.BATTERY_PLUGGED_USB -> "usb"
                    plugged == BatteryManager.BATTERY_PLUGGED_WIRELESS -> "wireless"
                    plugged > 0 -> "plugged"  // Any other charging type
                    plugged == 0 -> "unplugged"
                    else -> "unknown"
                }
                info["charging_state"] = chargingState

                // Calculate battery drain rate
                val currentTime = System.currentTimeMillis()
                val batteryDrainPctPerHour = if (initialBatteryLevel < 0.0f) {
                    // First call - initialize
                    initialBatteryLevel = batteryPct
                    initialBatteryTimestamp = currentTime
                    0.0f
                } else {
                    // Calculate drain rate
                    val elapsedHours = (currentTime - initialBatteryTimestamp) / (1000.0f * 3600.0f)
                    if (elapsedHours > 0.0f) {
                        (initialBatteryLevel - batteryPct) / elapsedHours
                    } else {
                        0.0f
                    }
                }
                info["battery_drain_pct_per_hour"] = batteryDrainPctPerHour

                // Device information
                info["device_brand"] = Build.BRAND
                info["device_model"] = Build.MODEL
                info["os_version"] = "Android ${Build.VERSION.RELEASE}"

                Log.d(pluginName, "Mobile device info collected successfully")
            } catch (e: Exception) {
                Log.e(pluginName, "Error collecting mobile device info: ${e.message}")
                // Return defaults on error
                info["memory_usage"] = -1
                info["device_temperature_celsius"] = -1.0f
                info["thermal_state"] = "unknown"
                info["battery_drain_pct_per_hour"] = -1.0f
                info["device_brand"] = ""
                info["device_model"] = ""
                info["os_version"] = ""
                info["total_ram_mb"] = -1
            }
        } ?: run {
            Log.e(pluginName, "Activity is null, cannot collect device info")
        }

        return info
    }

    companion object {
        // Static variables for battery drain calculation
        private var initialBatteryLevel: Float = -1.0f
        private var initialBatteryTimestamp: Long = 0L
    }

}
