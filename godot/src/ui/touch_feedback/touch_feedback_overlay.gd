extends CanvasLayer

## Touch-feedback debug tool.
##
## Draws a translucent circle at each active touch point, on a CanvasLayer above the
## entire game/UI. Registered as the `TouchFeedback` autoload. Disabled by default on
## every build; enabled at runtime via the `touch-feedback=true` deeplink param (see
## deep_link_router.gd). Never activates on production builds (see set_enabled).

const TouchFeedbackDrawScript := preload("res://src/ui/touch_feedback/touch_feedback_draw.gd")

## Above all game and UI CanvasLayers (which use small layer numbers).
const OVERLAY_LAYER := 128

# The draw node while enabled, null while disabled.
var _draw_node: Control = null


func _ready() -> void:
	layer = OVERLAY_LAYER
	follow_viewport_enabled = false


## Toggle the touch-feedback overlay at runtime. Idempotent; never enables on production.
func set_enabled(enable: bool) -> void:
	if enable and Global.is_production():
		return

	var currently_enabled := is_instance_valid(_draw_node)
	if currently_enabled == enable:
		return

	if enable:
		_draw_node = TouchFeedbackDrawScript.new()
		_draw_node.name = "TouchFeedbackDraw"
		_draw_node.set_anchors_preset(Control.PRESET_FULL_RECT)
		# Never intercept input: this overlay only observes touches, the game still receives them.
		_draw_node.mouse_filter = Control.MOUSE_FILTER_IGNORE
		add_child(_draw_node)
	else:
		_draw_node.queue_free()
		_draw_node = null
