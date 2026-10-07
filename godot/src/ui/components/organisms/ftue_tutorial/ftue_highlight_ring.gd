class_name FtueHighlightRing
extends Control

## Pulsing ring drawn around the control a tutorial step points at. Centred on its own position.

const COLOR := Color(0.875, 0.612, 1.0)
const LINE_WIDTH := 3.0
const STATE_SECONDS := 0.3
# Design sizes: the ring grows from 194 to 212 at its peak.
const PEAK_SCALE := 212.0 / 194.0

var diameter := 194.0
var ring_scale := 1.0:
	set(value):
		ring_scale = value
		queue_redraw()
var ring_alpha := 0.9:
	set(value):
		ring_alpha = value
		queue_redraw()

var _tween: Tween


func play(center: Vector2, ring_diameter: float) -> void:
	global_position = center
	diameter = ring_diameter
	show()
	if _tween != null:
		_tween.kill()
	ring_scale = 1.0
	ring_alpha = 0.9
	_tween = create_tween().set_loops().set_ease(Tween.EASE_OUT)
	_tween.tween_property(self, "ring_alpha", 0.4, STATE_SECONDS)
	_tween.tween_property(self, "ring_alpha", 0.9, STATE_SECONDS)
	_tween.parallel().tween_property(self, "ring_scale", PEAK_SCALE, STATE_SECONDS)
	_tween.tween_property(self, "ring_alpha", 0.5, STATE_SECONDS)
	_tween.parallel().tween_property(self, "ring_scale", 1.0, STATE_SECONDS)
	_tween.tween_property(self, "ring_alpha", 0.0, STATE_SECONDS)
	_tween.tween_property(self, "ring_alpha", 0.9, STATE_SECONDS)


func stop() -> void:
	if _tween != null:
		_tween.kill()
		_tween = null
	hide()


func _draw() -> void:
	draw_arc(
		Vector2.ZERO,
		diameter * ring_scale * 0.5,
		0.0,
		TAU,
		64,
		Color(COLOR, ring_alpha),
		LINE_WIDTH,
		true
	)
