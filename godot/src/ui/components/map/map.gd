extends Control

signal jump_to(tile: Vector2i)

@export var drag_enabled: bool = false

var parcel_to_jump: Vector2i
var mouse_tile: Vector2i
var last_mouse_tile: Vector2i

@onready var jump_in: ColorRect = %JumpIn

@onready var label_mouse_position = $Control_Tooltip/Label_MousePosition
@onready var control_tooltip = $Control_Tooltip
@onready var control_map_shader = %Control_MapShader


func _ready() -> void:
	jump_in.hide()


func _on_gui_input(event):
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


func _on_control_map_shader_on_move():
	jump_in.hide()


#function to call when menu is closed
func clear():
	control_tooltip.show()
	jump_in.hide()


# gdlint:ignore = async-function-name
func _on_control_map_shader_parcel_click(_parcel_position):
	mouse_tile = control_map_shader.get_parcel_from_mouse()
	mouse_tile = Vector2i(floor(mouse_tile.x), floor(mouse_tile.y))
	control_map_shader.set_selected_parcel(mouse_tile)
	parcel_to_jump = mouse_tile
	UiSounds.play_sound("mainmenu_tile_highlight")
	await jump_in.async_load_place_position(mouse_tile)


func _on_jump_in_jump_in(parcel_position: Vector2i, realm: String) -> void:
	var explorer = Global.get_explorer()
	if is_instance_valid(explorer):
		explorer.teleport_to(parcel_position, realm)
		jump_in.hide()
		explorer.hide_menu()
	else:
		Global.get_config().last_realm_joined = realm
		Global.get_config().last_parcel_position = parcel_position
		Global.get_config().add_place_to_last_places(parcel_position, realm)
		get_tree().change_scene_to_file("res://src/ui/explorer.tscn")
