extends Control

signal request_open_map

@onready var control_map_shader = $Panel_Background/Control_MapShader
@onready var label_parcel = $Panel_Background/Label_Parcel
@onready var panel_background = $Panel_Background

@onready var button_menu = $Button_Menu


func _ready():
	if Global.is_mobile:
		panel_background.hide()
		button_menu.show()
	else:
		panel_background.show()
		button_menu.hide()


func set_center_position(player_position: Vector2):
	control_map_shader.set_center_position(player_position)

	var parcel_position = Vector2i(floori(player_position.x), floori(player_position.y))
	control_map_shader.set_selected_parcel(parcel_position)
	label_parcel.text = str(parcel_position)


func _on_control_map_shader_gui_input(event):
	if (
		not Global.is_mobile
		&& event is InputEventMouseButton
		and event.pressed
		and event.button_index == MOUSE_BUTTON_LEFT
	):
		emit_signal("request_open_map")


func _on_button_menu_pressed():
	if Global.is_mobile:
		emit_signal("request_open_map")
