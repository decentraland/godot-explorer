extends Control

var angles: Array = [0, 60, 90, 120, 180]
var initial_angle: float

var tooltip_scene = preload("res://src/ui/components/pointer_tooltip/tooltip_label.tscn")

@onready var control_center = $Control_Center


func set_pointer_data(interacts_array: Array):
	for child in control_center.get_children():
		child.queue_free()
	var i = 0
	for interact in interacts_array:
		if i >= angles.size(): break
		var tooltip_scene_instance = tooltip_scene.instantiate()
		tooltip_scene_instance.set_position(Vector2(0, -95).rotated(deg_to_rad(angles[i])))
		var tooltip_position = tooltip_scene_instance.get_position()
		tooltip_scene_instance.set_position(Vector2(tooltip_position.x, tooltip_position.y - 20))
		control_center.add_child(tooltip_scene_instance)
		tooltip_scene_instance.set_tooltip_data(
			interact.get("text_pet_down", ""), interact.get("text_pet_up", ""), interact.get("action", "")
		)

		i = i + 1
