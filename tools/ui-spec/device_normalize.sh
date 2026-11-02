# Sourced by rn_capture.sh: pins the device so captures repeat (720x1600 at 320 dpi, no
# animations, fixed status bar clock). Needs `adb` to point at one device (ANDROID_SERIAL).
normalize_device() {
  adb shell wm size 720x1600
  adb shell wm density 320
  for scale in window_animation_scale transition_animation_scale animator_duration_scale; do
    adb shell settings put global "$scale" 0
  done
  adb shell settings put system font_scale 1.0
  # A slow emulator can ANR the launcher; its dialog would sit on top of the capture.
  adb shell settings put global hide_error_dialogs 1
  adb shell settings put global sysui_demo_allowed 1
  demo() { adb shell am broadcast -a com.android.systemui.demo -e command "$@" >/dev/null; }
  demo enter
  demo clock -e hhmm 1200
  demo battery -e level 100 -e plugged false
  demo network -e wifi show -e level 4
  demo notifications -e visible false
}
restore_device() {
  adb shell am broadcast -a com.android.systemui.demo -e command exit >/dev/null
  adb shell wm size reset
  adb shell wm density reset
  for scale in window_animation_scale transition_animation_scale animator_duration_scale; do
    adb shell settings put global "$scale" 1
  done
}
