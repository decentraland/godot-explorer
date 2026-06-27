extends Node

## BootInstrumentation
##
## Autoload registered FIRST in project.godot. Owns the boot clock, the
## Sentry breadcrumb chain, and (on non-prod builds) an on-screen progress
## console rendered over the splash. Drives diagnosis of iOS WatchdogTermination
## crashes (Sentry: GODOT-EXPLORER-3B, GH #1532, #1980).
##
## Usage from anywhere: BootInstrumentation.mark("step_name")
##
## On non-prod, marks render as ephemeral console lines (auto-disappear after
## 3s). Triple-finger touch toggles the console on/off.

signal step_marked(step_name: String, elapsed_ms: int)

const _BOOT_TAG := "last_boot_step"
const _ITEM_LIFETIME_S := 3.0
const _MAX_VISIBLE_ITEMS := 20
# RAM indicator lives OUTSIDE the safe area, pinned to the top-right corner
# in the channel between the notch / dynamic island and the screen edge.
# Pinning to the right (vs. using a width fraction) guarantees we clear the
# dynamic island regardless of how wide it is — which a 75% fraction did NOT
# (its right edge sits at ~71% of the screen width on iPhone 14/15).
const _RAM_INDICATOR_EDGE_PADDING_PX := 8.0

var _start_time_ms: int = Time.get_ticks_msec()
var _layer: CanvasLayer = null
var _safe_container: SafeMarginContainer = null
var _vbox: VBoxContainer = null
var _ram_indicator: RamUsageIndicator = null
var _active_touches: Dictionary = {}
var _console_visible: bool = true


func _ready() -> void:
	if not DclGlobal.is_production():
		_install_debug_console.call_deferred()
	# Self-mark as the first explicit step. Earlier than this we can't reliably
	# call mark() because the autoload Node isn't in the tree during
	# ProjectMainLoop._initialize. Elapsed-ms is from script-load time (set on
	# _start_time_ms), so the duration covers the engine boot + Sentry init.
	mark("boot_instrumentation._ready")


## Record a named boot step. Prints, sets the `last_boot_step` Sentry tag, and
## emits step_marked so any subsequent crash (especially the OS-level
## WatchdogTermination, which carries no stack) is labeled with the last step
## the GDScript side reached.
func mark(step_name: String) -> void:
	var elapsed := Time.get_ticks_msec() - _start_time_ms
	print("[Startup] %s: %dms" % [step_name, elapsed])

	# SentrySDK.set_tag attaches a `last_boot_step` tag to every subsequent
	# event, including the iOS WatchdogTermination one (which carries no
	# stack trace). The print itself is also picked up by Sentry's logger
	# breadcrumb conversion when enabled, so the step name reaches Sentry
	# both ways.
	SentrySDK.set_tag(_BOOT_TAG, step_name)

	step_marked.emit(step_name, elapsed)


## Milliseconds elapsed since boot start (this autoload's script-load time; it
## registers first in project.godot). Use for startup-relative telemetry timing
## instead of the removed Global._startup_time.
func boot_elapsed_ms() -> int:
	return Time.get_ticks_msec() - _start_time_ms


func _install_debug_console() -> void:
	_layer = CanvasLayer.new()
	# Above modal_manager (100) and version_gate (99) so the console is never
	# occluded by an overlay shown while async_boot is still running.
	_layer.layer = 128

	# SafeMarginContainer applies the OS / emulated safe area as margins and
	# tracks window-size changes itself. The VBox inside fills the entire
	# safe-area rect (full-rect, top-to-bottom) so console lines can extend
	# all the way down the screen.
	_safe_container = SafeMarginContainer.new()
	_safe_container.name = "BootInstrumentationConsole"
	_safe_container.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_safe_container.set_anchors_preset(Control.PRESET_FULL_RECT)
	_layer.add_child(_safe_container)

	_vbox = VBoxContainer.new()
	_vbox.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_vbox.add_theme_constant_override("separation", 2)
	_safe_container.add_child(_vbox)

	# RAM indicator: sibling of the safe container so it's NOT inset by the
	# safe-area margins. Positioned in the top-right band between the notch /
	# dynamic island and the right edge; re-positioned on window resize so it
	# follows portrait <-> landscape rotations.
	_ram_indicator = RamUsageIndicator.new()
	_layer.add_child(_ram_indicator)

	add_child(_layer)

	_position_ram_indicator()
	# Reposition on every layout change. window.size_changed fires on resize
	# (including the resize that accompanies orientation flips); Global emits
	# orientation_changed explicitly when the OS swaps portrait <-> landscape.
	# We listen to both because on iOS one fires before the safe-area inset
	# has updated and the other after — and we want to be correct either way.
	var window := get_window()
	if window != null:
		window.size_changed.connect(_on_layout_changed)
	Global.orientation_changed.connect(_on_orientation_changed)

	step_marked.connect(_on_step_marked)


func _on_orientation_changed(_is_portrait: bool) -> void:
	_on_layout_changed()


func _on_layout_changed() -> void:
	# Defer one idle frame so DisplayServer / Global.get_safe_area() has time
	# to publish the post-rotation safe-area rectangle before we sample it.
	_position_ram_indicator.call_deferred()


func _position_ram_indicator() -> void:
	if not is_instance_valid(_ram_indicator):
		return
	var window := get_window()
	if window == null:
		return
	var window_size := Vector2(window.size)
	# Global.get_safe_area() honors --emulate-ios / --emulate-android and so
	# already returns the simulated insets on desktop test runs. iOS portrait
	# is top=177, others=0; iOS landscape is left=right=177 (dynamic island
	# can rotate either way); Android landscape is left-only.
	var safe_area: Rect2 = Global.get_safe_area()
	var indicator_size: float = RamUsageIndicator.SIZE_PX
	var padding: float = _RAM_INDICATOR_EDGE_PADDING_PX

	var top_inset: float = safe_area.position.y
	var left_inset: float = safe_area.position.x
	var right_inset: float = window_size.x - (safe_area.position.x + safe_area.size.x)
	var bottom_inset: float = window_size.y - (safe_area.position.y + safe_area.size.y)
	var max_inset: float = maxf(maxf(top_inset, bottom_inset), maxf(left_inset, right_inset))

	# Place the indicator in the largest unsafe band — i.e. wherever the
	# notch / dynamic-island actually lives in the current orientation —
	# biased toward the top-right corner of that band. The tiebreaker order
	# (right > top > left > bottom) is what lands iOS landscape (where
	# left and right insets are equal) on the right side.
	var center: Vector2
	if right_inset > 0.0 and is_equal_approx(right_inset, max_inset):
		# Right band: indicator centered in the band, near the top.
		center = Vector2(window_size.x - right_inset * 0.5, indicator_size * 0.5 + padding)
	elif top_inset > 0.0 and is_equal_approx(top_inset, max_inset):
		# Top band: indicator at the right edge, vertically centered in band.
		center = Vector2(window_size.x - indicator_size * 0.5 - padding, top_inset * 0.5)
	elif left_inset > 0.0 and is_equal_approx(left_inset, max_inset):
		# Left band: indicator centered in the band, near the top.
		center = Vector2(left_inset * 0.5, indicator_size * 0.5 + padding)
	elif bottom_inset > 0.0 and is_equal_approx(bottom_inset, max_inset):
		# Bottom band (unusual): right-aligned, centered vertically in band.
		center = Vector2(
			window_size.x - indicator_size * 0.5 - padding, window_size.y - bottom_inset * 0.5
		)
	else:
		# No safe-area insets (desktop window): fall back to the absolute
		# top-right corner with a small padding.
		center = Vector2(
			window_size.x - indicator_size * 0.5 - padding, indicator_size * 0.5 + padding
		)

	# Clamp so the circle stays within the visible window (defends against
	# very small inset bands or unusual window sizes).
	center.x = clampf(center.x, indicator_size * 0.5, window_size.x - indicator_size * 0.5)
	center.y = clampf(center.y, indicator_size * 0.5, window_size.y - indicator_size * 0.5)

	_ram_indicator.position = Vector2(
		center.x - indicator_size * 0.5, center.y - indicator_size * 0.5
	)
	_ram_indicator.size = Vector2(indicator_size, indicator_size)


func _on_step_marked(step_name: String, elapsed_ms: int) -> void:
	if not is_instance_valid(_vbox):
		return
	var line := Label.new()
	line.text = "[%dms] %s" % [elapsed_ms, step_name]
	line.add_theme_color_override("font_color", Color.WHITE)
	line.add_theme_color_override("font_outline_color", Color.BLACK)
	line.add_theme_constant_override("outline_size", 4)
	line.mouse_filter = Control.MOUSE_FILTER_IGNORE
	line.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	_vbox.add_child(line)

	# Cap the buffer so a long boot doesn't grow the VBox indefinitely.
	# queue_free() defers removal to end-of-frame, so we must remove_child
	# eagerly — otherwise get_child_count never drops and the loop spins.
	while _vbox.get_child_count() > _MAX_VISIBLE_ITEMS:
		var oldest := _vbox.get_child(0)
		_vbox.remove_child(oldest)
		oldest.queue_free()

	_expire_line.call_deferred(line)


# gdlint:ignore = async-function-name
func _expire_line(line: Label) -> void:
	await get_tree().create_timer(_ITEM_LIFETIME_S).timeout
	if is_instance_valid(line):
		line.queue_free()


# Triple-finger tap (any three fingers down simultaneously) toggles the
# console. We listen at _input so it works regardless of which scene owns
# focus. Mouse / keyboard fallback: F8 also toggles.
func _input(event: InputEvent) -> void:
	if event is InputEventScreenTouch:
		var touch := event as InputEventScreenTouch
		if touch.pressed:
			_active_touches[touch.index] = true
			if _active_touches.size() >= 3:
				_toggle_console()
				_active_touches.clear()
		else:
			_active_touches.erase(touch.index)
	elif event is InputEventKey and event.pressed and not event.echo:
		var key := event as InputEventKey
		if key.keycode == KEY_F8:
			_toggle_console()


func _toggle_console() -> void:
	_console_visible = not _console_visible
	if is_instance_valid(_safe_container):
		_safe_container.visible = _console_visible
	if is_instance_valid(_ram_indicator):
		_ram_indicator.visible = _console_visible
