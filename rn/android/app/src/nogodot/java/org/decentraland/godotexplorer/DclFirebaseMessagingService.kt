package org.decentraland.godotexplorer

import android.app.Service
import android.content.Intent

/** Declared in the shared manifest; the no-engine build has no Firebase, so FCM never starts it. */
class DclFirebaseMessagingService : Service() {
  override fun onBind(intent: Intent?) = null
}
