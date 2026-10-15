package com.dclrn.push

import android.Manifest
import android.app.AlarmManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.app.ActivityManager
import android.content.pm.PackageManager
import android.os.Build
import android.util.Log
import androidx.core.app.NotificationManagerCompat
import androidx.core.content.ContextCompat
import com.facebook.react.bridge.Arguments
import com.facebook.react.bridge.Promise
import com.facebook.react.bridge.ReactApplicationContext
import com.facebook.react.bridge.ReactContextBaseJavaModule
import com.facebook.react.bridge.ReactMethod
import com.facebook.react.modules.core.PermissionAwareActivity
import com.facebook.react.modules.core.PermissionListener
import com.google.firebase.messaging.FirebaseMessaging
import java.util.Locale
import java.util.TimeZone
import org.decentraland.godotexplorer.NotificationReceiver
import org.decentraland.godotexplorer.PushTokenStore

/**
 * The OS side of push for the shell: the FCM token, the notification permission and local
 * reminders. The remote pushes themselves are drawn by the plugin's DclFirebaseMessagingService,
 * which runs with no JS and no engine; local reminders fire through the plugin's NotificationReceiver.
 */
class DclPushModule(private val context: ReactApplicationContext) :
    ReactContextBaseJavaModule(context) {

  override fun getName() = "DclPush"

  @ReactMethod
  fun getToken(promise: Promise) {
    try {
      FirebaseMessaging.getInstance().token.addOnCompleteListener { task ->
        val token = if (task.isSuccessful) task.result ?: "" else PushTokenStore.getToken(context)
        if (token.isNotEmpty()) PushTokenStore.saveToken(context, token)
        promise.resolve(token)
      }
    } catch (e: Exception) {
      Log.w(TAG, "FCM token unavailable: ${e.message}")
      promise.resolve(PushTokenStore.getToken(context))
    }
  }

  @ReactMethod
  fun hasPermission(promise: Promise) {
    promise.resolve(permissionGranted())
  }

  @ReactMethod
  fun requestPermission(promise: Promise) {
    if (Build.VERSION.SDK_INT < Build.VERSION_CODES.TIRAMISU || permissionGranted()) {
      promise.resolve(permissionGranted())
      return
    }
    val activity = context.currentActivity as? PermissionAwareActivity
    if (activity == null) {
      promise.resolve(false)
      return
    }
    val listener = PermissionListener { requestCode, _, grantResults ->
      if (requestCode != REQUEST_CODE) return@PermissionListener false
      promise.resolve(grantResults.firstOrNull() == PackageManager.PERMISSION_GRANTED)
      true
    }
    activity.requestPermissions(arrayOf(Manifest.permission.POST_NOTIFICATIONS), REQUEST_CODE, listener)
  }

  @ReactMethod
  fun deviceInfo(promise: Promise) {
    val info = Arguments.createMap()
    val version =
        try {
          context.packageManager.getPackageInfo(context.packageName, 0).versionName ?: ""
        } catch (e: Exception) {
          ""
        }
    info.putString("appVersion", version)
    info.putString("locale", Locale.getDefault().toString())
    info.putString("timezone", TimeZone.getDefault().id)
    promise.resolve(info)
  }

  /** What the core's device-support rule needs; empty strings where the OS does not say. */
  @ReactMethod
  fun deviceFacts(promise: Promise) {
    val facts = Arguments.createMap()
    val memory = ActivityManager.MemoryInfo()
    (context.getSystemService(Context.ACTIVITY_SERVICE) as ActivityManager).getMemoryInfo(memory)
    facts.putDouble("totalRamMb", memory.totalMem / (1024.0 * 1024.0))
    facts.putString("model", Build.MODEL ?: "")
    facts.putString("brand", Build.MANUFACTURER ?: "")
    facts.putString("osVersion", "Android ${Build.VERSION.RELEASE}")
    facts.putString("socModel", if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) Build.SOC_MODEL else "")
    facts.putString("boardPlatform", systemProperty("ro.board.platform"))
    facts.putString("hardware", Build.HARDWARE ?: "")
    promise.resolve(facts)
  }

  private fun systemProperty(name: String): String =
      try {
        Class.forName("android.os.SystemProperties")
            .getMethod("get", String::class.java, String::class.java)
            .invoke(null, name, "") as String
      } catch (e: Exception) {
        ""
      }

  /** A reminder `delaySeconds` from now; tapping it opens `deepLink` (empty: just the app). */
  @ReactMethod
  fun schedule(id: String, title: String, body: String, delaySeconds: Int, deepLink: String, promise: Promise) {
    try {
      val alarms = context.getSystemService(Context.ALARM_SERVICE) as AlarmManager
      val at = System.currentTimeMillis() + delaySeconds * 1000L
      val exact = Build.VERSION.SDK_INT < Build.VERSION_CODES.S || alarms.canScheduleExactAlarms()
      val pending = reminderIntent(id, title, body, deepLink)
      if (exact) {
        alarms.setExactAndAllowWhileIdle(AlarmManager.RTC_WAKEUP, at, pending)
      } else {
        alarms.setAndAllowWhileIdle(AlarmManager.RTC_WAKEUP, at, pending)
      }
      promise.resolve(true)
    } catch (e: Exception) {
      Log.e(TAG, "Could not schedule $id: ${e.message}")
      promise.resolve(false)
    }
  }

  @ReactMethod
  fun cancel(id: String, promise: Promise) {
    try {
      val alarms = context.getSystemService(Context.ALARM_SERVICE) as AlarmManager
      alarms.cancel(reminderIntent(id, "", "", ""))
      promise.resolve(true)
    } catch (e: Exception) {
      promise.resolve(false)
    }
  }

  private fun reminderIntent(id: String, title: String, body: String, deepLink: String): PendingIntent {
    val key = id.hashCode()
    val intent =
        Intent(context, NotificationReceiver::class.java).apply {
          action = NotificationReceiver.NOTIFICATION_ACTION
          putExtra(NotificationReceiver.EXTRA_NOTIFICATION_ID, key)
          putExtra(NotificationReceiver.EXTRA_NOTIFICATION_STRING_ID, id)
          putExtra(NotificationReceiver.EXTRA_TITLE, title)
          putExtra(NotificationReceiver.EXTRA_BODY, body)
          if (deepLink.isNotEmpty()) putExtra(NotificationReceiver.EXTRA_DEEP_LINK, deepLink)
        }
    return PendingIntent.getBroadcast(
        context, key, intent, PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE)
  }

  private fun permissionGranted(): Boolean =
      if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
        ContextCompat.checkSelfPermission(context, Manifest.permission.POST_NOTIFICATIONS) ==
            PackageManager.PERMISSION_GRANTED
      } else {
        NotificationManagerCompat.from(context).areNotificationsEnabled()
      }

  companion object {
    private const val TAG = "DclPush"
    private const val REQUEST_CODE = 7301
  }
}
