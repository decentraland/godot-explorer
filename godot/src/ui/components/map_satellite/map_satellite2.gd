extends Control

signal jump_to(tile: Vector2i)

@export var drag_enabled: bool = false
@onready var map_control: Control = %MapControl

var parcel_to_jump: Vector2i
var mouse_tile: Vector2i
var last_mouse_tile: Vector2i



func _ready() -> void:
	pass

func _on_gui_input(event):
	if event is InputEventMouseButton:
		if not event.pressed:
			var zoom_value = map_control.zoom_value

			if event.button_index == MOUSE_BUTTON_WHEEL_UP:
				if zoom_value < 48:
					map_control.set_zoom(zoom_value + 1)
			if event.button_index == MOUSE_BUTTON_WHEEL_DOWN:
				if zoom_value > 1:
					map_control.set_zoom(zoom_value - 1)

	if event is InputEventMouseMotion:
		mouse_tile = map_control.get_parcel_from_mouse()
		mouse_tile = Vector2i(floor(mouse_tile.x), floor(mouse_tile.y))

		

		if last_mouse_tile != mouse_tile:
			last_mouse_tile = mouse_tile
			map_control.set_selected_parcel(mouse_tile)
