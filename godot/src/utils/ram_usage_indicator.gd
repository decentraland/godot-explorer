class_name RamUsageIndicator
extends Control

## Circular system-RAM usage indicator for the boot debug overlay.
##
## Polls OS.get_memory_info() once a second and renders used / physical as a
## ring chart (light background ring + teal progress arc) with the percentage
## centered inside. Lives inside the BootInstrumentation overlay so it inherits
## that overlay's visibility (non-prod only; toggled by triple-finger / F8).

const SIZE_PX := 64.0
const _RADIUS := 26.0
const _RING_WIDTH := 5.0
const _SAMPLE_INTERVAL_S := 1.0

const _RING_BG_COLOR := Color(0.94, 0.91, 0.86, 0.9)
const _RING_FG_COLOR := Color(0.30, 0.48, 0.52, 1.0)

var _percent: float = 0.0
var _label: Label = null


func _ready() -> void:
	custom_minimum_size = Vector2(SIZE_PX, SIZE_PX)
	mouse_filter = Control.MOUSE_FILTER_IGNORE

	_label = Label.new()
	_label.text = "--%"
	_label.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_label.add_theme_color_override("font_color", Color.WHITE)
	_label.add_theme_color_override("font_outline_color", Color.BLACK)
	_label.add_theme_constant_override("outline_size", 4)
	_label.set_anchors_preset(Control.PRESET_FULL_RECT)
	_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	_label.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
	add_child(_label)

	var timer := Timer.new()
	timer.wait_time = _SAMPLE_INTERVAL_S
	timer.autostart = true
	timer.timeout.connect(_sample)
	add_child(timer)

	_sample()


func _sample() -> void:
	var mem := OS.get_memory_info()
	var total: int = int(mem.get("physical", 0))
	if total <= 0:
		return
	# Some platforms (notably iOS) leave `available` at 0; fall back to `free`.
	var available: int = int(mem.get("available", 0))
	if available <= 0:
		available = int(mem.get("free", 0))
	var used: int = maxi(0, total - available)
	_percent = clampf((float(used) / float(total)) * 100.0, 0.0, 100.0)
	if is_instance_valid(_label):
		_label.text = "%d%%" % int(round(_percent))
	queue_redraw()


func _draw() -> void:
	var center := size * 0.5
	draw_arc(center, _RADIUS, 0.0, TAU, 64, _RING_BG_COLOR, _RING_WIDTH, true)
	if _percent > 0.0:
		# Start at 12 o'clock and sweep clockwise.
		var start_angle := -PI * 0.5
		var end_angle := start_angle + (TAU * _percent / 100.0)
		draw_arc(center, _RADIUS, start_angle, end_angle, 64, _RING_FG_COLOR, _RING_WIDTH, true)
