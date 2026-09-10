extends Control

## Horizontal gap between the crosshair and a tooltip's left edge. It positions a lone tooltip
## outright, and is also the floor for the ring below: a tooltip is a wide left-anchored box, not
## a point, so at 0 and 180 degrees the ring would otherwise sit its left edge on the crosshair.
const MIN_OFFSET_X: float = 40.0
## Vertical gap left between two neighbouring tooltips in the ring.
const PILL_GAP: float = 8.0

var angles: Array = [0, 60, 90, 120, 180]
var initial_angle: float

var tooltip_scene = preload("res://src/ui/components/molecules/pointer_tooltip/tooltip_label.tscn")

@onready var control_center = %Control_Center
@onready var panel_center = %Panel_Crosshair


func set_global_cursor_position(_position: Vector2):
	control_center.set_global_position(_position)
	panel_center.set_global_position(_position - panel_center.size / 2)


func _get_centered_angles(count: int) -> Array:
	var center := angles.size() / 2
	var start := center - count / 2
	return angles.slice(start, start + count)


func set_pointer_data(interacts_array: Array):
	for child in control_center.get_children():
		child.queue_free()
	var count = min(interacts_array.size(), angles.size())
	var used_angles = _get_centered_angles(count)
	var i = 0
	for interact in interacts_array:
		if i >= count:
			break
		var tooltip_scene_instance = tooltip_scene.instantiate()
		var pill_height: float = tooltip_scene_instance.size.y
		var offset := Vector2(MIN_OFFSET_X, 0.0)
		if count > 1:
			# Neighbours on the ring are half a radius apart vertically, so the radius has to be
			# twice the pill height (plus the gap) or stacked tooltips overlap each other.
			var radius := 2.0 * (pill_height + PILL_GAP)
			offset = Vector2(0.0, -radius).rotated(deg_to_rad(used_angles[i]))
			offset.x = maxf(offset.x, MIN_OFFSET_X)
			# Neighbours clear each other by only PILL_GAP, so a vertical tap ring would overlap
			# one and let a tap fire the wrong prompt. Only a lone tooltip grows vertically.
			tooltip_scene_instance.tap_grow_y = 0.0
		# set_position takes the top-left corner; the offsets above are to the tooltip's centre.
		offset.y -= pill_height / 2.0
		tooltip_scene_instance.set_position(offset)
		control_center.add_child(tooltip_scene_instance)
		tooltip_scene_instance.set_tooltip_data(
			interact.get("text_pet_down", ""),
			interact.get("text_pet_up", ""),
			interact.get("action", "")
		)

		i = i + 1
