extends Control

signal request_open_map

@onready var control_map_shader = $Panel_Background/Control_MapShader
@onready var label_parcel = $Panel_Background/Label_Parcel


func set_center_position(player_position: Vector2):
	control_map_shader.set_center_position(player_position)

	var parcel_position = Vector2i(player_position)
	control_map_shader.set_selected_parcel(parcel_position)
	label_parcel.text = str(parcel_position)


func _on_control_map_shader_gui_input(event):
	if event is InputEventMouseButton and event.pressed and event.button_index == MOUSE_BUTTON_LEFT:
		emit_signal("request_open_map")
