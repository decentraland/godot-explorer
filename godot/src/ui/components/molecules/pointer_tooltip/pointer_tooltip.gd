extends Control

## A lone tooltip is placed at 90 degrees, where the ring below degenerates to a single point, so
## this is really a horizontal gap between the crosshair and the tooltip's left edge -- negative
## only because it shares the ring's expression, where a negative y means "above the crosshair".
## The pill's height does not affect it.
const SINGLE_OFFSET_X: float = -36.0
## Radius of the ring several tooltips are laid out on. Neighbours are separated by half of it, so
## it grew by twice the 52px -> 60px pill height change (#2710) to keep the clearance they had.
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
		var radius = RADIUS_MULTI if interacts_array.size() > 1 else SINGLE_OFFSET_X
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
