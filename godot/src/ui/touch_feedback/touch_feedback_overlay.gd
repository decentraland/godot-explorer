extends CanvasLayer

## Dev-only touch-feedback debug tool.
##
## Draws a translucent circle at each active touch point, on a CanvasLayer above the
## entire game/UI. Registered as the `TouchFeedback` autoload. It only activates on
## development builds (debug build and not production) — mirroring the DebugWs gate — so
## it never ships to end users.

const TouchFeedbackDrawScript := preload("res://src/ui/touch_feedback/touch_feedback_draw.gd")

## Above all game and UI CanvasLayers (which use small layer numbers).
const OVERLAY_LAYER := 128


func _ready() -> void:
	# Dev-only: active on debug builds, never in production (same gate as DebugWs).
	if not (OS.is_debug_build() and not Global.is_production()):
		return

	layer = OVERLAY_LAYER
	follow_viewport_enabled = false

	var draw_node: Control = TouchFeedbackDrawScript.new()
	draw_node.name = "TouchFeedbackDraw"
	draw_node.set_anchors_preset(Control.PRESET_FULL_RECT)
	# Never intercept input: this overlay only observes touches, the game still receives them.
	draw_node.mouse_filter = Control.MOUSE_FILTER_IGNORE
	add_child(draw_node)
