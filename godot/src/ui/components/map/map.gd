extends Control

signal jump_to(tile: Vector2i)

@export var drag_enabled: bool = false

var parcel_to_jump: Vector2i
var mouse_tile: Vector2i
var last_mouse_tile: Vector2i

@onready var control_jump_to = $Control_JumpTo
@onready var label_mouse_position = $Control_Tooltip/Label_MousePosition
@onready var label_parcel_position = $Control_JumpTo/Panel_JumpTo/VBoxContainer/Label_ParcelPosition
@onready var control_tooltip = $Control_Tooltip
@onready var control_map_shader = $Control_MapShader


func _gui_input(event):
	if event is InputEventMouseButton:
		if not event.pressed:
			var zoom_value = control_map_shader.zoom_value

			if event.button_index == MOUSE_BUTTON_WHEEL_UP:
				if zoom_value < 48:
					control_map_shader.set_zoom(zoom_value + 1)
			if event.button_index == MOUSE_BUTTON_WHEEL_DOWN:
				if zoom_value > 1:
					control_map_shader.set_zoom(zoom_value - 1)

	if event is InputEventMouseMotion:
		mouse_tile = control_map_shader.get_parcel_from_mouse()
		mouse_tile = Vector2i(floor(mouse_tile.x), floor(mouse_tile.y))

		control_tooltip.position = event.position
		control_tooltip.show()

		if last_mouse_tile != mouse_tile:
			last_mouse_tile = mouse_tile
			label_mouse_position.text = str(mouse_tile)
			control_map_shader.set_selected_parcel(mouse_tile)


func _on_button_pressed():
	jump_to.emit(parcel_to_jump)
	control_jump_to.hide()


func _on_control_map_shader_on_move():
	control_jump_to.hide()


#function to call when menu is closed
func clear():
	control_tooltip.show()
	control_jump_to.hide()


func _on_control_map_shader_parcel_click(_parcel_position):
	mouse_tile = control_map_shader.get_parcel_from_mouse()
	mouse_tile = Vector2i(floor(mouse_tile.x), floor(mouse_tile.y))
	parcel_to_jump = mouse_tile
	control_jump_to.position = get_global_mouse_position()
	label_parcel_position.text = str(mouse_tile)
	control_jump_to.show()
