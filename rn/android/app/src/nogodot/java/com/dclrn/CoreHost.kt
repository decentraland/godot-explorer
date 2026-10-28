package com.dclrn

import android.app.Application

/** Without the engine library there is no core to install; the UI runs on the mock core. */
object CoreHost {
  fun install(app: Application) = Unit
}
