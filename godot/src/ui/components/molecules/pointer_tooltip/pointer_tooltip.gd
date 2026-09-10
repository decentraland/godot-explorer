extends Control

## Distance from the crosshair to a tooltip's *center*, negative because the ring is laid out
## upwards. Both were retuned when the pill grew from 52px to 60px tall (#2710): the single-hint
## value by half the growth, so the pill's bottom edge keeps the same gap to the crosshair, and the
## multi-hint value by twice the growth, since neighbours are separated by half the radius.
const RADIUS_SINGLE: float = -40.0
const RADIUS_MULTI: float = -106.0

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
		var radius = RADIUS_MULTI if interacts_array.size() > 1 else RADIUS_SINGLE
		tooltip_scene_instance.set_position(
			Vector2(0, radius - (4 * count)).rotated(deg_to_rad(used_angles[i]))
		)
		var tooltip_position = tooltip_scene_instance.get_position()
		tooltip_scene_instance.set_position(
			Vector2(tooltip_position.x, tooltip_position.y - tooltip_scene_instance.size.y / 2)
		)
		control_center.add_child(tooltip_scene_instance)
		tooltip_scene_instance.set_tooltip_data(
			interact.get("text_pet_down", ""),
			interact.get("text_pet_up", ""),
			interact.get("action", "")
		)

		i = i + 1
