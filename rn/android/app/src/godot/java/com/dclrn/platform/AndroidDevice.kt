package com.dclrn.platform

import android.app.ActivityManager
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.os.BatteryManager
import android.os.Build
import android.os.Handler
import android.os.Looper
import android.util.Log
import java.io.File
import org.decentraland.core.host.DeviceInfo
import org.decentraland.core.host.DeviceReadings
import org.decentraland.core.host.DeviceService

/**
 * Device readings for the engine: the static facts once, thermal and battery on demand. Called
 * from the engine's thread; nothing here may throw into Rust. Memory is left to the engine, which
 * reads its own PSS off the game loop.
 */
class AndroidDevice(private val context: Context) : DeviceService {

  override fun info(): DeviceInfo =
      try {
        val memory = ActivityManager.MemoryInfo()
        (context.getSystemService(Context.ACTIVITY_SERVICE) as ActivityManager).getMemoryInfo(memory)
        val api31 = Build.VERSION.SDK_INT >= Build.VERSION_CODES.S
        DeviceInfo(
            brand = Build.BRAND ?: "",
            model = Build.MODEL ?: "",
            osVersion = "Android ${Build.VERSION.RELEASE}",
            totalRamMb = (memory.totalMem / (1024 * 1024)).toInt(),
            // Chipset ids match a device against the Play exclusion / below-minspec tables
            // (#2936 / #2935); ro.board.platform and Build.HARDWARE cover API < 31.
            socModel = if (api31) Build.SOC_MODEL ?: "" else "",
            socManufacturer = if (api31) Build.SOC_MANUFACTURER ?: "" else "",
            boardPlatform = systemProperty("ro.board.platform"),
            hardware = Build.HARDWARE ?: "",
            // Android has no CPU-name API; the primary ABI is the closest OS description.
            processorType = Build.SUPPORTED_ABIS?.firstOrNull() ?: "",
            processorMaxFreqMhz = maxCpuFreqMhz(),
        )
      } catch (e: Exception) {
        Log.e(TAG, "Error collecting device info: ${e.message}")
        DeviceInfo("", "", "", -1, "", "", "", "", "", -1)
      }

  /** A sticky-intent read: cheap enough for the game loop. */
  override fun readings(): DeviceReadings =
      try {
        val battery = context.registerReceiver(null, IntentFilter(Intent.ACTION_BATTERY_CHANGED))
        // Tenths of a degree Celsius.
        val tenths = battery?.getIntExtra(BatteryManager.EXTRA_TEMPERATURE, -1) ?: -1
        val celsius = if (tenths > 0) tenths / 10.0f else -1.0f
        val thermal =
            when {
              celsius < 0 -> "unknown"
              celsius < 40.0f -> "nominal"
              celsius < 45.0f -> "fair"
              celsius < 50.0f -> "serious"
              else -> "critical"
            }
        val level = battery?.getIntExtra(BatteryManager.EXTRA_LEVEL, -1) ?: -1
        val scale = battery?.getIntExtra(BatteryManager.EXTRA_SCALE, -1) ?: -1
        val percent = if (level >= 0 && scale > 0) level * 100.0f / scale else -1.0f
        val plugged = battery?.getIntExtra(BatteryManager.EXTRA_PLUGGED, -1) ?: -1
        val status = battery?.getIntExtra(BatteryManager.EXTRA_STATUS, -1) ?: -1
        val charging =
            when {
              status == BatteryManager.BATTERY_STATUS_FULL -> "full"
              plugged == BatteryManager.BATTERY_PLUGGED_AC -> "plugged"
              plugged == BatteryManager.BATTERY_PLUGGED_USB -> "usb"
              plugged == BatteryManager.BATTERY_PLUGGED_WIRELESS -> "wireless"
              plugged > 0 -> "plugged"
              plugged == 0 -> "unplugged"
              else -> "unknown"
            }
        DeviceReadings(-1, celsius, thermal, percent, charging)
      } catch (e: Exception) {
        Log.e(TAG, "Error collecting thermal state: ${e.message}")
        DeviceReadings(-1, -1.0f, "unknown", -1.0f, "unknown")
      }

  /** The engine's main loop runs on the render thread; an ANR needs the UI thread blocked. */
  override fun blockUiThread(ms: UInt): Boolean {
    Handler(Looper.getMainLooper()).post {
      try {
        Thread.sleep(ms.toLong())
      } catch (_: InterruptedException) {}
    }
    return true
  }

  private fun maxCpuFreqMhz(): Int {
    var maxKhz = -1L
    File("/sys/devices/system/cpu").listFiles { f -> f.name.matches(Regex("cpu\\d+")) }?.forEach { core ->
      try {
        val khz = File(core, "cpufreq/cpuinfo_max_freq").readText().trim().toLong()
        if (khz > maxKhz) maxKhz = khz
      } catch (e: Exception) {
        // An unreadable core is skipped; the others may still report.
      }
    }
    return if (maxKhz > 0) (maxKhz / 1000).toInt() else -1
  }

  private fun systemProperty(key: String): String =
      try {
        Class.forName("android.os.SystemProperties")
            .getMethod("get", String::class.java)
            .invoke(null, key) as String
      } catch (e: Exception) {
        ""
      }

  companion object {
    private const val TAG = "AndroidDevice"
  }
}
