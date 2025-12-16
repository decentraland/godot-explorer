package org.decentraland.godotexplorer

import android.Manifest
import android.app.Activity
import android.app.ActivityManager
import android.app.AlarmManager
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.ActivityNotFoundException
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.content.pm.ActivityInfo
import android.content.pm.PackageManager
import android.graphics.Bitmap
import android.net.Uri
import android.os.BatteryManager
import android.os.Build
import android.provider.CalendarContract
import android.util.Log
import android.webkit.WebResourceRequest
import android.widget.FrameLayout
import android.widget.TextView
import android.webkit.WebView
import android.webkit.WebViewClient
import android.widget.Toast
import androidx.browser.customtabs.CustomTabsIntent
import androidx.browser.customtabs.CustomTabsService
import androidx.core.app.ActivityCompat
import androidx.core.content.ContextCompat
import org.decentraland.godotexplorer.NotificationReceiver
import org.godotengine.godot.Dictionary
import org.godotengine.godot.Godot
import org.godotengine.godot.plugin.GodotPlugin
import org.godotengine.godot.plugin.UsedByGodot
import java.io.File
import java.io.FileOutputStream

class GodotAndroidPlugin(godot: Godot) : GodotPlugin(godot) {

    private var webView: WebView? = null
    private var isWebViewOpen: Boolean = false
    private var overlayLayout: FrameLayout? = null

    // ExoPlayer management
    private val exoPlayers = mutableMapOf<Int, org.decentraland.godotexplorer.ExoPlayerWrapper>()
    private var nextPlayerId = 1
    // Notification database instance
    private var notificationDatabase: NotificationDatabase? = null

    private val customPackageNames = arrayOf(
        "com.android.chrome",        // Google Chrome
        "org.mozilla.firefox",       // Mozilla Firefox
        "com.microsoft.emmx",        // Microsoft Edge
        "com.brave.browser",         // Brave Browser
        "com.opera.browser",         // Opera Browser
        "com.opera.mini.native",     // Opera Mini
        "com.sec.android.app.sbrowser" // Samsung Internet
    )

    override fun onGodotSetupCompleted() {
        super.onGodotSetupCompleted()
        // Initialize notification database
        activity?.let {
            notificationDatabase = NotificationDatabase(it.applicationContext)
            Log.d(pluginName, "Notification database initialized")
        }
    }

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
                // Get total RAM
                val activityManager = ctx.getSystemService(Context.ACTIVITY_SERVICE) as ActivityManager
                val memInfo = ActivityManager.MemoryInfo()
                activityManager.getMemoryInfo(memInfo)
                val totalRamMB = memInfo.totalMem / (1024 * 1024)
                info["total_ram_mb"] = totalRamMB.toInt()

                // Device information (static)
                info["device_brand"] = Build.BRAND
                info["device_model"] = Build.MODEL
                info["os_version"] = "Android ${Build.VERSION.RELEASE}"

                Log.d(pluginName, "Mobile device info collected successfully")
            } catch (e: Exception) {
                Log.e(pluginName, "Error collecting mobile device info: ${e.message}")
                // Return defaults on error
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

    @UsedByGodot
    fun getMobileMetrics(): Dictionary {
        val metrics = Dictionary()

        activity?.let { ctx ->
            try {
                // Get fresh memory info using ActivityManager (updated each call)
                val activityManager = ctx.getSystemService(Context.ACTIVITY_SERVICE) as ActivityManager
                val memInfo = android.os.Debug.MemoryInfo()
                android.os.Debug.getMemoryInfo(memInfo)

                // For Godot apps, we need Native + Graphics + Code + Stack + Java heap
                // Using individual components gives us a fresh, accurate total
                val nativeHeapKB = memInfo.nativePss
                val dalvikHeapKB = memInfo.dalvikPss
                val otherPssKB = memInfo.otherPss

                val totalMemoryKB = nativeHeapKB + dalvikHeapKB + otherPssKB
                val totalMemoryMB = totalMemoryKB / 1024

                metrics["memory_usage"] = totalMemoryMB

                // Get battery information
                val batteryIntentFilter = IntentFilter(Intent.ACTION_BATTERY_CHANGED)
                val batteryStatus = ctx.registerReceiver(null, batteryIntentFilter)

                // Battery temperature (in tenths of a degree Celsius)
                val temperature = batteryStatus?.getIntExtra(BatteryManager.EXTRA_TEMPERATURE, -1) ?: -1
                val temperatureCelsius = if (temperature > 0) temperature / 10.0f else -1.0f
                metrics["device_temperature_celsius"] = temperatureCelsius

                // Approximate thermal state based on temperature
                val thermalState = when {
                    temperatureCelsius < 0 -> "unknown"
                    temperatureCelsius < 40.0f -> "nominal"
                    temperatureCelsius < 45.0f -> "fair"
                    temperatureCelsius < 50.0f -> "serious"
                    else -> "critical"
                }
                metrics["thermal_state"] = thermalState

                // Battery level
                val batteryLevel = batteryStatus?.getIntExtra(BatteryManager.EXTRA_LEVEL, -1) ?: -1
                val batteryScale = batteryStatus?.getIntExtra(BatteryManager.EXTRA_SCALE, -1) ?: -1
                val batteryPercent = if (batteryLevel >= 0 && batteryScale > 0) {
                    (batteryLevel.toFloat() / batteryScale.toFloat()) * 100.0f
                } else {
                    -1.0f
                }
                metrics["battery_percent"] = batteryPercent

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
                metrics["charging_state"] = chargingState

                Log.d(pluginName, "Mobile metrics collected successfully")
            } catch (e: Exception) {
                Log.e(pluginName, "Error collecting mobile metrics: ${e.message}")
                // Return defaults on error
                metrics["memory_usage"] = -1
                metrics["device_temperature_celsius"] = -1.0f
                metrics["thermal_state"] = "unknown"
                metrics["battery_percent"] = -1.0f
                metrics["charging_state"] = "unknown"
            }
        } ?: run {
            Log.e(pluginName, "Activity is null, cannot collect metrics")
        }

        return metrics
    }

    @UsedByGodot
    fun getLaunchIntentData(): Dictionary {
        val result = Dictionary()

        activity?.let { act ->
            val intent = act.intent
            val action = intent?.action
            val data = intent?.dataString

            result["action"] = action ?: ""
            result["data"] = data ?: ""
            result["extras"] = Dictionary()

            // Copy extras (if any)
            intent?.extras?.keySet()?.forEach { key ->
                intent.extras?.get(key)?.let { value ->
                    (result["extras"] as Dictionary)[key] = value.toString()
                }
            }
        } ?: run {
            Log.e(pluginName, "Activity is null, cannot retrieve intent")
            result["error"] = "Activity is null"
        }

        return result
    }

    @UsedByGodot
    fun addCalendarEvent(
        title: String,
        description: String,
        startTimeMillis: Long,
        endTimeMillis: Long,
        location: String
    ): Boolean {
        val act = activity ?: run {
            Log.e(pluginName, "Activity is null, cannot add calendar event")
            return false
        }

        // Check for calendar permissions
        val hasReadPermission = ContextCompat.checkSelfPermission(
            act,
            Manifest.permission.READ_CALENDAR
        ) == PackageManager.PERMISSION_GRANTED

        val hasWritePermission = ContextCompat.checkSelfPermission(
            act,
            Manifest.permission.WRITE_CALENDAR
        ) == PackageManager.PERMISSION_GRANTED

        // Request permissions if needed (Android 6.0+)
        if (!hasReadPermission || !hasWritePermission) {
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.M) {
                ActivityCompat.requestPermissions(
                    act,
                    arrayOf(Manifest.permission.READ_CALENDAR, Manifest.permission.WRITE_CALENDAR),
                    CALENDAR_PERMISSION_REQUEST_CODE
                )
                Log.d(pluginName, "Requesting calendar permissions")
                // Return false as permissions need to be granted first
                // User should call this function again after granting permissions
                return false
            }
        }

        try {
            // Create an intent to insert a calendar event
            val intent = Intent(Intent.ACTION_INSERT).apply {
                data = CalendarContract.Events.CONTENT_URI
                putExtra(CalendarContract.Events.TITLE, title)
                putExtra(CalendarContract.Events.DESCRIPTION, description)
                putExtra(CalendarContract.Events.EVENT_LOCATION, location)
                putExtra(CalendarContract.EXTRA_EVENT_BEGIN_TIME, startTimeMillis)
                putExtra(CalendarContract.EXTRA_EVENT_END_TIME, endTimeMillis)
                // Allow user to select calendar and edit the event
                putExtra(CalendarContract.Events.AVAILABILITY, CalendarContract.Events.AVAILABILITY_BUSY)
            }

            // Launch the calendar app with the event details
            act.startActivity(intent)
            Log.d(pluginName, "Calendar event intent launched successfully")
            return true
        } catch (e: ActivityNotFoundException) {
            Log.e(pluginName, "No calendar app found: ${e.message}")
            showMessage("No calendar app found")
            return false
        } catch (e: Exception) {
            Log.e(pluginName, "Error adding calendar event: ${e.message}")
            return false
        }
    }

    @UsedByGodot
    fun shareText(text: String): Boolean {
        val act = activity ?: run {
            Log.e(pluginName, "Activity is null, cannot share text")
            return false
        }

        try {
            val shareIntent = Intent().apply {
                action = Intent.ACTION_SEND
                type = "text/plain"
                putExtra(Intent.EXTRA_TEXT, text)
            }

            val chooserIntent = Intent.createChooser(shareIntent, "Share via")
            act.startActivity(chooserIntent)
            Log.d(pluginName, "Share text intent launched successfully")
            return true
        } catch (e: Exception) {
            Log.e(pluginName, "Error sharing text: ${e.message}")
            return false
        }
    }

    @UsedByGodot
    fun shareTextWithImage(text: String, width: Int, height: Int, imageData: ByteArray): Boolean {
        val act = activity ?: run {
            Log.e(pluginName, "Activity is null, cannot share text with image")
            return false
        }

        try {
            if (width <= 0 || height <= 0) {
                Log.e(pluginName, "Invalid image dimensions: ${width}x${height}")
                return false
            }

            if (imageData.size != width * height * 4) {
                Log.e(pluginName, "Invalid image data size. Expected ${width * height * 4}, got ${imageData.size}")
                return false
            }

            // Convert RGBA byte array to Android Bitmap
            val bitmap = Bitmap.createBitmap(width, height, Bitmap.Config.ARGB_8888)

            // Copy pixel data to Bitmap
            val pixels = IntArray(width * height)
            for (i in 0 until width * height) {
                val offset = i * 4
                val r = imageData[offset].toInt() and 0xFF
                val g = imageData[offset + 1].toInt() and 0xFF
                val b = imageData[offset + 2].toInt() and 0xFF
                val a = imageData[offset + 3].toInt() and 0xFF
                pixels[i] = (a shl 24) or (r shl 16) or (g shl 8) or b
            }
            bitmap.setPixels(pixels, 0, width, 0, 0, width, height)

            // Save bitmap to cache directory
            val cacheDir = act.cacheDir
            val imageFile = File(cacheDir, "share_image_${System.currentTimeMillis()}.png")
            FileOutputStream(imageFile).use { out ->
                bitmap.compress(Bitmap.CompressFormat.PNG, 100, out)
            }

            // Try to use FileProvider if configured, otherwise fall back to file URI
            val imageUri = try {
                androidx.core.content.FileProvider.getUriForFile(
                    act,
                    "${act.packageName}.fileprovider",
                    imageFile
                )
            } catch (e: IllegalArgumentException) {
                // FileProvider not configured, use file URI as fallback
                Log.w(pluginName, "FileProvider not configured, using file URI: ${e.message}")
                Uri.fromFile(imageFile)
            }

            val shareIntent = Intent().apply {
                action = Intent.ACTION_SEND
                type = "image/png"
                putExtra(Intent.EXTRA_TEXT, text)
                putExtra(Intent.EXTRA_STREAM, imageUri)
                addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
            }

            val chooserIntent = Intent.createChooser(shareIntent, "Share via")
            act.startActivity(chooserIntent)
            Log.d(pluginName, "Share text with image intent launched successfully")

            // Clean up the temporary file after a delay (let the share complete first)
            android.os.Handler(android.os.Looper.getMainLooper()).postDelayed({
                imageFile.delete()
            }, 5000)

            return true
        } catch (e: Exception) {
            Log.e(pluginName, "Error sharing text with image: ${e.message}")
            e.printStackTrace()
            return false
        }
    }

    // =============================================================================
    // LOCAL NOTIFICATIONS
    // =============================================================================

    /**
     * Request notification permission for Android 13+ (API 33+).
     * For older versions, returns true immediately as no runtime permission is needed.
     */
    @UsedByGodot
    fun requestNotificationPermission(): Boolean {
        val act = activity ?: run {
            Log.e(pluginName, "Activity is null, cannot request notification permission")
            return false
        }

        // Android 13+ requires POST_NOTIFICATIONS runtime permission
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            val hasPermission = ContextCompat.checkSelfPermission(
                act,
                Manifest.permission.POST_NOTIFICATIONS
            ) == PackageManager.PERMISSION_GRANTED

            if (!hasPermission) {
                ActivityCompat.requestPermissions(
                    act,
                    arrayOf(Manifest.permission.POST_NOTIFICATIONS),
                    NOTIFICATION_PERMISSION_REQUEST_CODE
                )
                Log.d(pluginName, "Requesting POST_NOTIFICATIONS permission")
                return false
            }
            return true
        }

        // For Android 12 and below, permission is automatically granted
        return true
    }

    /**
     * Check if notification permission is granted.
     */
    @UsedByGodot
    fun hasNotificationPermission(): Boolean {
        val act = activity ?: return false

        return if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            ContextCompat.checkSelfPermission(
                act,
                Manifest.permission.POST_NOTIFICATIONS
            ) == PackageManager.PERMISSION_GRANTED
        } else {
            true
        }
    }

    /**
     * Create a notification channel (required for Android 8.0+).
     * This should be called before scheduling any notifications.
     */
    @UsedByGodot
    fun createNotificationChannel(
        channelId: String,
        channelName: String,
        channelDescription: String
    ): Boolean {
        val ctx = activity ?: run {
            Log.e(pluginName, "Activity is null, cannot create notification channel")
            return false
        }

        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            try {
                val notificationManager = ctx.getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager

                val channel = NotificationChannel(
                    channelId,
                    channelName,
                    NotificationManager.IMPORTANCE_DEFAULT
                ).apply {
                    description = channelDescription
                    enableVibration(true)
                    enableLights(true)
                }

                notificationManager.createNotificationChannel(channel)
                Log.d(pluginName, "Notification channel created: $channelId")
                return true
            } catch (e: Exception) {
                Log.e(pluginName, "Error creating notification channel: ${e.message}")
                return false
            }
        }

        // For Android 7.1 and below, channels are not needed
        return true
    }

    /**
     * Schedule a local notification to be displayed after a delay.
     *
     * @param notificationId Unique ID for this notification (used for cancellation)
     * @param title Notification title
     * @param body Notification body text
     * @param delaySeconds Delay in seconds before showing the notification
     * @return true if scheduled successfully, false otherwise
     */
    @UsedByGodot
    fun scheduleLocalNotification(
        notificationId: String,
        title: String,
        body: String,
        delaySeconds: Int
    ): Boolean {
        val ctx = activity ?: run {
            Log.e(pluginName, "Activity is null, cannot schedule notification")
            return false
        }

        try {
            // Convert string ID to integer hash for Android
            val intId = notificationId.hashCode()

            // Fetch image blob and deep link from database if available
            val imageBlob = notificationDatabase?.getNotificationImageBlob(notificationId)
            val deepLink = notificationDatabase?.getNotificationDeepLink(notificationId) ?: ""

            Log.d(pluginName, "Scheduling notification: id=$notificationId, deepLink=$deepLink")

            // Create intent for NotificationReceiver
            val intent = Intent(ctx, NotificationReceiver::class.java).apply {
                action = NotificationReceiver.NOTIFICATION_ACTION
                putExtra(NotificationReceiver.EXTRA_NOTIFICATION_ID, intId)
                putExtra(NotificationReceiver.EXTRA_NOTIFICATION_STRING_ID, notificationId)
                putExtra(NotificationReceiver.EXTRA_TITLE, title)
                putExtra(NotificationReceiver.EXTRA_BODY, body)
                if (imageBlob != null) {
                    putExtra(NotificationReceiver.EXTRA_IMAGE_BLOB, imageBlob)
                }
                if (deepLink.isNotEmpty()) {
                    putExtra(NotificationReceiver.EXTRA_DEEP_LINK, deepLink)
                }
            }

            val pendingIntent = PendingIntent.getBroadcast(
                ctx,
                intId,
                intent,
                PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
            )

            // Schedule with AlarmManager
            val alarmManager = ctx.getSystemService(Context.ALARM_SERVICE) as AlarmManager
            val triggerTimeMillis = System.currentTimeMillis() + (delaySeconds * 1000L)

            // Use setExactAndAllowWhileIdle for precise timing
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.M) {
                alarmManager.setExactAndAllowWhileIdle(
                    AlarmManager.RTC_WAKEUP,
                    triggerTimeMillis,
                    pendingIntent
                )
            } else {
                alarmManager.setExact(
                    AlarmManager.RTC_WAKEUP,
                    triggerTimeMillis,
                    pendingIntent
                )
            }

            Log.d(pluginName, "Local notification scheduled: id=$notificationId (hash=$intId), delay=${delaySeconds}s, hasImage=${imageBlob != null}")
            return true
        } catch (e: Exception) {
            Log.e(pluginName, "Error scheduling local notification: ${e.message}")
            e.printStackTrace()
            return false
        }
    }

    /**
     * Cancel a scheduled local notification.
     *
     * @param notificationId The ID of the notification to cancel
     * @return true if cancelled successfully, false otherwise
     */
    @UsedByGodot
    fun cancelLocalNotification(notificationId: String): Boolean {
        val ctx = activity ?: run {
            Log.e(pluginName, "Activity is null, cannot cancel notification")
            return false
        }

        try {
            val intId = notificationId.hashCode()

            // Cancel the pending alarm
            val intent = Intent(ctx, NotificationReceiver::class.java).apply {
                action = NotificationReceiver.NOTIFICATION_ACTION
            }

            val pendingIntent = PendingIntent.getBroadcast(
                ctx,
                intId,
                intent,
                PendingIntent.FLAG_NO_CREATE or PendingIntent.FLAG_IMMUTABLE
            )

            if (pendingIntent != null) {
                val alarmManager = ctx.getSystemService(Context.ALARM_SERVICE) as AlarmManager
                alarmManager.cancel(pendingIntent)
                pendingIntent.cancel()
                Log.d(pluginName, "Local notification cancelled: id=$notificationId (hash=$intId)")
            } else {
                Log.w(pluginName, "No pending notification found with id=$notificationId")
            }

            // Also remove from notification tray if already displayed
            val notificationManager = ctx.getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
            notificationManager.cancel(intId)

            return true
        } catch (e: Exception) {
            Log.e(pluginName, "Error cancelling local notification: ${e.message}")
            return false
        }
    }

    /**
     * Cancel all scheduled local notifications.
     */
    @UsedByGodot
    fun cancelAllLocalNotifications(): Boolean {
        val ctx = activity ?: run {
            Log.e(pluginName, "Activity is null, cannot cancel all notifications")
            return false
        }

        try {
            // Clear all notifications from the notification tray
            val notificationManager = ctx.getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
            notificationManager.cancelAll()

            Log.d(pluginName, "All local notifications cancelled")
            return true
        } catch (e: Exception) {
            Log.e(pluginName, "Error cancelling all local notifications: ${e.message}")
            return false
        }
    }

    // =============================================================================
    // DATABASE API - Unified notification queue management
    // =============================================================================

    /**
     * Insert or replace a notification in the database.
     * Part of the unified database API (Phase 3).
     */
    @UsedByGodot
    fun dbInsertNotification(
        id: String,
        title: String,
        body: String,
        triggerTimestamp: Long,
        isScheduled: Int = 0,
        data: String = "",
        imageBase64: String = ""
    ): Boolean {
        val db = notificationDatabase ?: run {
            Log.e(pluginName, "Database not initialized")
            return false
        }

        // Convert base64 string to ByteArray if provided
        val imageBlob = if (imageBase64.isNotEmpty()) {
            try {
                android.util.Base64.decode(imageBase64, android.util.Base64.DEFAULT)
            } catch (e: Exception) {
                Log.e(pluginName, "Error decoding image base64: ${e.message}")
                null
            }
        } else {
            null
        }

        return db.insertNotification(
            id,
            title,
            body,
            triggerTimestamp,
            isScheduled,
            if (data.isEmpty()) null else data,
            imageBlob
        )
    }

    /**
     * Update notification fields in the database.
     * Part of the unified database API (Phase 3).
     */
    @UsedByGodot
    fun dbUpdateNotification(id: String, updates: Dictionary): Boolean {
        val db = notificationDatabase ?: run {
            Log.e(pluginName, "Database not initialized")
            return false
        }

        return db.updateNotification(id, updates)
    }

    /**
     * Delete a notification from the database.
     * Part of the unified database API (Phase 3).
     */
    @UsedByGodot
    fun dbDeleteNotification(id: String): Boolean {
        val db = notificationDatabase ?: run {
            Log.e(pluginName, "Database not initialized")
            return false
        }

        return db.deleteNotification(id)
    }

    /**
     * Query notifications from the database with filters.
     * Part of the unified database API (Phase 3).
     *
     * @param whereClause SQL WHERE clause, e.g. "is_scheduled = 0 AND trigger_timestamp > 1699564800"
     * @param orderBy SQL ORDER BY clause, e.g. "trigger_timestamp ASC"
     * @param limit Maximum results, or -1 for no limit
     */
    @UsedByGodot
    fun dbQueryNotifications(whereClause: String = "", orderBy: String = "", limit: Int = -1): Array<Dictionary> {
        val db = notificationDatabase ?: run {
            Log.e(pluginName, "Database not initialized")
            return emptyArray()
        }

        return db.queryNotifications(whereClause, orderBy, limit)
    }

    /**
     * Get count of notifications matching filter.
     * Part of the unified database API (Phase 3).
     */
    @UsedByGodot
    fun dbCountNotifications(whereClause: String = ""): Int {
        val db = notificationDatabase ?: run {
            Log.e(pluginName, "Database not initialized")
            return 0
        }

        return db.countNotifications(whereClause)
    }

    /**
     * Clear expired notifications from the database.
     * Part of the unified database API (Phase 3).
     *
     * @param currentTimestamp Current Unix timestamp (seconds)
     * @return Number of deleted notifications
     */
    @UsedByGodot
    fun dbClearExpired(currentTimestamp: Long): Int {
        val db = notificationDatabase ?: run {
            Log.e(pluginName, "Database not initialized")
            return 0
        }

        return db.clearExpired(currentTimestamp)
    }

    /**
     * Mark a notification as scheduled/unscheduled in the database.
     * Part of the unified database API (Phase 3).
     */
    @UsedByGodot
    fun dbMarkScheduled(id: String, isScheduled: Boolean): Boolean {
        val db = notificationDatabase ?: run {
            Log.e(pluginName, "Database not initialized")
            return false
        }

        return db.markScheduled(id, isScheduled)
    }

    /**
     * Get a single notification by ID from the database.
     * Part of the unified database API (Phase 3).
     */
    @UsedByGodot
    fun dbGetNotification(id: String): Dictionary {
        val db = notificationDatabase ?: run {
            Log.e(pluginName, "Database not initialized")
            return Dictionary()
        }

        return db.getNotification(id)
    }

    /**
     * Clear all notifications from the database.
     * Part of the unified database API (Phase 3).
     *
     * @return Number of deleted notifications
     */
    @UsedByGodot
    fun dbClearAll(): Int {
        val db = notificationDatabase ?: run {
            Log.e(pluginName, "Database not initialized")
            return 0
        }

        return db.clearAll()
    }

    // =============================================================================
    // OS NOTIFICATION API - Renamed methods for clarity
    // =============================================================================

    /**
     * Schedule a notification with the OS (AlarmManager).
     * This is the low-level OS API (Phase 3).
     */
    @UsedByGodot
    fun osScheduleNotification(
        notificationId: String,
        title: String,
        body: String,
        delaySeconds: Int
    ): Boolean {
        // This is the same as the existing scheduleLocalNotification
        return scheduleLocalNotification(notificationId, title, body, delaySeconds)
    }

    /**
     * Cancel a notification from the OS (AlarmManager).
     * This is the low-level OS API (Phase 3).
     */
    @UsedByGodot
    fun osCancelNotification(notificationId: String): Boolean {
        // This is the same as the existing cancelLocalNotification
        return cancelLocalNotification(notificationId)
    }

    /**
     * Get the image blob for a specific notification (as base64 string).
     * This is separate from queryNotifications() for performance.
     *
     * @param id Notification ID
     * @return Base64 encoded image data, or empty string if no image
     */
    @UsedByGodot
    fun dbGetNotificationImageBlob(id: String): String {
        val db = notificationDatabase ?: run {
            Log.e(pluginName, "Database not initialized")
            return ""
        }

        val imageBlob = db.getNotificationImageBlob(id) ?: return ""

        return try {
            android.util.Base64.encodeToString(imageBlob, android.util.Base64.DEFAULT)
        } catch (e: Exception) {
            Log.e(pluginName, "Error encoding image blob to base64: ${e.message}")
            ""
        }
    }

    /**
     * Get IDs of notifications currently scheduled with the OS.
     * Part of the unified OS API (Phase 3).
     *
     * Note: Android doesn't provide a direct way to query AlarmManager,
     * so we maintain a registry in the database via is_scheduled flag.
     */
    @UsedByGodot
    fun osGetScheduledIds(): Array<String> {
        val db = notificationDatabase ?: run {
            Log.e(pluginName, "Database not initialized")
            return emptyArray()
        }

        // Query notifications marked as scheduled
        val scheduled = db.queryNotifications("is_scheduled = 1", "", -1)
        return scheduled.map { it["id"].toString() }.toTypedArray()
    }

    companion object {
        private const val CALENDAR_PERMISSION_REQUEST_CODE = 1001
        private const val NOTIFICATION_PERMISSION_REQUEST_CODE = 1002
    }


    // ==================== ExoPlayer Methods ====================

    @UsedByGodot
    fun createExoPlayer(): Int {
        return try {
            val playerId = nextPlayerId++
            val act = activity ?: return -1

            var player: org.decentraland.godotexplorer.ExoPlayerWrapper? = null
            val latch = java.util.concurrent.CountDownLatch(1)

            runOnUiThread {
                try {
                    player = org.decentraland.godotexplorer.ExoPlayerWrapper(act, playerId)
                } catch (e: Exception) {
                    Log.e(pluginName, "Error creating ExoPlayer: ${e.message}", e)
                } finally {
                    latch.countDown()
                }
            }

            latch.await()

            val createdPlayer = player ?: return -1
            exoPlayers[playerId] = createdPlayer
            playerId
        } catch (e: Exception) {
            Log.e(pluginName, "Error creating ExoPlayer: ${e.message}", e)
            -1
        }
    }

    @UsedByGodot
    fun exoPlayerGetLastError(playerId: Int): String {
        return exoPlayers[playerId]?.getInitError() ?: "No error or player not found"
    }

    @UsedByGodot
    fun exoPlayerInitSurface(playerId: Int, width: Int, height: Int): Int {
        return exoPlayers[playerId]?.initializeSurface(width, height) ?: -1
    }

    @UsedByGodot
    fun exoPlayerUpdateTexture(playerId: Int): Boolean {
        return exoPlayers[playerId]?.updateTexture() ?: false
    }

    @UsedByGodot
    fun exoPlayerSetSourceUrl(playerId: Int, url: String): Boolean {
        return exoPlayers[playerId]?.setSourceUrl(url) ?: false
    }

    @UsedByGodot
    fun exoPlayerSetSourceLocal(playerId: Int, filePath: String): Boolean {
        return exoPlayers[playerId]?.setSourceLocal(filePath) ?: false
    }

    @UsedByGodot
    fun exoPlayerPlay(playerId: Int) {
        exoPlayers[playerId]?.play()
    }

    @UsedByGodot
    fun exoPlayerPause(playerId: Int) {
        exoPlayers[playerId]?.pause()
    }

    @UsedByGodot
    fun exoPlayerStop(playerId: Int) {
        exoPlayers[playerId]?.stop()
    }

    @UsedByGodot
    fun exoPlayerSetPosition(playerId: Int, positionMs: Long) {
        exoPlayers[playerId]?.setPosition(positionMs)
    }

    @UsedByGodot
    fun exoPlayerGetPosition(playerId: Int): Long {
        return exoPlayers[playerId]?.getPosition() ?: 0L
    }

    @UsedByGodot
    fun exoPlayerGetDuration(playerId: Int): Long {
        return exoPlayers[playerId]?.getDuration() ?: 0L
    }

    @UsedByGodot
    fun exoPlayerIsPlaying(playerId: Int): Boolean {
        return exoPlayers[playerId]?.isPlaying() ?: false
    }

    @UsedByGodot
    fun exoPlayerGetVideoWidth(playerId: Int): Int {
        return exoPlayers[playerId]?.getVideoWidth() ?: 0
    }

    @UsedByGodot
    fun exoPlayerGetVideoHeight(playerId: Int): Int {
        return exoPlayers[playerId]?.getVideoHeight() ?: 0
    }

    @UsedByGodot
    fun exoPlayerHasVideoSizeChanged(playerId: Int): Boolean {
        return exoPlayers[playerId]?.hasVideoSizeChanged() ?: false
    }

    @UsedByGodot
    fun exoPlayerGetTextureWidth(playerId: Int): Int {
        return exoPlayers[playerId]?.getTextureWidth() ?: 0
    }

    @UsedByGodot
    fun exoPlayerGetTextureHeight(playerId: Int): Int {
        return exoPlayers[playerId]?.getTextureHeight() ?: 0
    }

    @UsedByGodot
    fun exoPlayerSetVolume(playerId: Int, volume: Float) {
        exoPlayers[playerId]?.setVolume(volume)
    }

    @UsedByGodot
    fun exoPlayerGetVolume(playerId: Int): Float {
        return exoPlayers[playerId]?.getVolume() ?: 1.0f
    }

    @UsedByGodot
    fun exoPlayerSetLooping(playerId: Int, loop: Boolean) {
        exoPlayers[playerId]?.setLooping(loop)
    }

    @UsedByGodot
    fun exoPlayerGetInfo(playerId: Int): String {
        return exoPlayers[playerId]?.getPlayerInfo() ?: "Player not found"
    }

    @UsedByGodot
    fun exoPlayerGetPixelData(playerId: Int): ByteArray {
        return exoPlayers[playerId]?.getPixelData() ?: ByteArray(0)
    }

    // ==================== ExoPlayer GPU Mode Methods ====================

    /**
     * Check if the player is using GPU mode (HardwareBuffer).
     * @return true if GPU mode is active, false if using CPU mode
     */
    @UsedByGodot
    fun exoPlayerIsGpuMode(playerId: Int): Boolean {
        return exoPlayers[playerId]?.isGpuMode() ?: false
    }

    /**
     * Check if a new HardwareBuffer frame is available (GPU mode only).
     * @return true if a new frame is available for GPU texture update
     */
    @UsedByGodot
    fun exoPlayerHasNewHardwareBuffer(playerId: Int): Boolean {
        return exoPlayers[playerId]?.hasNewHardwareBuffer() ?: false
    }

    /**
     * Get the native AHardwareBuffer* pointer for the current frame (GPU mode only).
     * This pointer can be passed to Godot's ExternalTexture for zero-copy GPU rendering.
     *
     * After calling this, the frame is marked as consumed and exoPlayerHasNewHardwareBuffer()
     * will return false until a new frame arrives from the video decoder.
     *
     * @return native AHardwareBuffer* pointer as a long (0 if not available or not in GPU mode)
     */
    @UsedByGodot
    fun exoPlayerAcquireHardwareBufferPtr(playerId: Int): Long {
        val player = exoPlayers[playerId] ?: return 0L
        return if (android.os.Build.VERSION.SDK_INT >= android.os.Build.VERSION_CODES.Q) {
            player.acquireHardwareBufferPtr()
        } else {
            0L
        }
    }

    @UsedByGodot
    fun exoPlayerRelease(playerId: Int): Boolean {
        val player = exoPlayers.remove(playerId) ?: return false
        player.release()
        return true
    }

    @UsedByGodot
    fun exoPlayerReleaseAll() {
        exoPlayers.values.forEach { it.release() }
        exoPlayers.clear()
    }

    // ==================== Memory Monitoring ====================

    /**
     * Get detailed memory information for the app.
     * Returns a Dictionary with memory stats in MB.
     */
    @UsedByGodot
    fun getMemoryInfo(): Dictionary {
        val dict = Dictionary()
        try {
            val activityManager = activity?.getSystemService(Context.ACTIVITY_SERVICE) as? ActivityManager
            if (activityManager != null) {
                val memInfo = ActivityManager.MemoryInfo()
                activityManager.getMemoryInfo(memInfo)

                // System-wide memory
                dict["system_total_mb"] = memInfo.totalMem / (1024.0 * 1024.0)
                dict["system_available_mb"] = memInfo.availMem / (1024.0 * 1024.0)
                dict["system_low_memory"] = memInfo.lowMemory
                dict["system_threshold_mb"] = memInfo.threshold / (1024.0 * 1024.0)

                // App-specific memory (requires Debug.MemoryInfo)
                val pids = intArrayOf(android.os.Process.myPid())
                val processMemInfo = activityManager.getProcessMemoryInfo(pids)
                if (processMemInfo.isNotEmpty()) {
                    val appMem = processMemInfo[0]
                    // Total PSS (Proportional Set Size) - best measure of actual memory use
                    dict["app_total_pss_mb"] = appMem.totalPss / 1024.0
                    // Private dirty - memory that can't be shared
                    dict["app_private_dirty_mb"] = appMem.totalPrivateDirty / 1024.0
                    // Shared dirty - memory shared with other processes
                    dict["app_shared_dirty_mb"] = appMem.totalSharedDirty / 1024.0

                    // Native heap (C/C++ allocations including Godot engine)
                    dict["native_pss_mb"] = appMem.nativePss / 1024.0
                    dict["native_private_dirty_mb"] = appMem.nativePrivateDirty / 1024.0

                    // Dalvik/ART heap (Java/Kotlin objects)
                    dict["dalvik_pss_mb"] = appMem.dalvikPss / 1024.0
                    dict["dalvik_private_dirty_mb"] = appMem.dalvikPrivateDirty / 1024.0

                    // Graphics memory (GPU allocations, textures, etc.)
                    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.M) {
                        // These are summaries that include GL, EGL, and Vulkan memory
                        dict["graphics_pss_mb"] = appMem.getMemoryStat("summary.graphics")?.toDouble()?.div(1024.0) ?: 0.0
                        dict["gl_pss_mb"] = appMem.getMemoryStat("summary.gl")?.toDouble()?.div(1024.0) ?: 0.0
                    }
                }

                // Runtime memory info
                val runtime = Runtime.getRuntime()
                dict["java_max_heap_mb"] = runtime.maxMemory() / (1024.0 * 1024.0)
                dict["java_total_heap_mb"] = runtime.totalMemory() / (1024.0 * 1024.0)
                dict["java_free_heap_mb"] = runtime.freeMemory() / (1024.0 * 1024.0)
                dict["java_used_heap_mb"] = (runtime.totalMemory() - runtime.freeMemory()) / (1024.0 * 1024.0)

                // Native heap from Debug
                dict["native_heap_size_mb"] = android.os.Debug.getNativeHeapSize() / (1024.0 * 1024.0)
                dict["native_heap_allocated_mb"] = android.os.Debug.getNativeHeapAllocatedSize() / (1024.0 * 1024.0)
                dict["native_heap_free_mb"] = android.os.Debug.getNativeHeapFreeSize() / (1024.0 * 1024.0)
            }
        } catch (e: Exception) {
            Log.e(pluginName, "Error getting memory info: ${e.message}")
            dict["error"] = e.message ?: "Unknown error"
        }
        return dict
    }

    /**
     * Force a garbage collection (for testing memory leaks).
     * Note: This only suggests GC, doesn't guarantee immediate collection.
     */
    @UsedByGodot
    fun forceGarbageCollection() {
        System.gc()
        Runtime.getRuntime().gc()
    }

    /**
     * Get a formatted string summary of memory usage for display.
     */
    @UsedByGodot
    fun getMemorySummary(): String {
        val memInfo = getMemoryInfo()
        val sb = StringBuilder()
        sb.appendLine("=== ANDROID MEMORY ===")
        sb.appendLine("App Total PSS: %.1f MB".format(memInfo.getOrDefault("app_total_pss_mb", 0.0)))
        sb.appendLine("Native Heap: %.1f / %.1f MB".format(
            memInfo.getOrDefault("native_heap_allocated_mb", 0.0),
            memInfo.getOrDefault("native_heap_size_mb", 0.0)
        ))
        sb.appendLine("Java Heap: %.1f / %.1f MB".format(
            memInfo.getOrDefault("java_used_heap_mb", 0.0),
            memInfo.getOrDefault("java_max_heap_mb", 0.0)
        ))
        sb.appendLine("Graphics: %.1f MB".format(memInfo.getOrDefault("graphics_pss_mb", 0.0)))
        sb.appendLine("System Available: %.1f MB".format(memInfo.getOrDefault("system_available_mb", 0.0)))
        return sb.toString()
    }

}
