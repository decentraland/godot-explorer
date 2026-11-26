package org.decentraland.godotexplorer

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent

/** Declared in the shared manifest; without the engine build nothing schedules a reminder. */
class NotificationReceiver : BroadcastReceiver() {
  override fun onReceive(context: Context, intent: Intent) = Unit
}
