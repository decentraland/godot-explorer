extends Control

## Tracks active touch points and draws one circle per finger. Hosted by the
## TouchFeedback CanvasLayer (production-only); see touch_feedback_overlay.gd.

const RADIUS := 44.0
const FILL_COLOR := Color(1.0, 1.0, 1.0, 0.22)
const RING_COLOR := Color(1.0, 1.0, 1.0, 0.85)
const RING_WIDTH := 3.0
const RING_POINTS := 32

# Active fingers: touch index (int) -> screen position (Vector2).
var _touches := {}


func _input(event: InputEvent) -> void:
	# _input fires before GUI handling and we never consume the event, so the game keeps
	# receiving every touch normally.
	if event is InputEventScreenTouch:
		if event.pressed:
			_touches[event.index] = event.position
		else:
			_touches.erase(event.index)
		queue_redraw()
	elif event is InputEventScreenDrag:
		_touches[event.index] = event.position
		queue_redraw()


func _draw() -> void:
	for touch_position in _touches.values():
		draw_circle(touch_position, RADIUS, FILL_COLOR)
		draw_arc(touch_position, RADIUS, 0.0, TAU, RING_POINTS, RING_COLOR, RING_WIDTH, true)
