
extends Control

signal click_on_tile(tile: Vector2i)

@export var drag_enabled: bool = false
@export var zoom_enabled: bool = false
@onready var map_rect = %ColorRect

var map_size: Vector2
var top_left_offset: Vector2i

const MAX_15B = 1 << 15
const MAX_16B = 1 << 16
	
func unsigned16_to_signed(unsigned):
	return (unsigned + MAX_15B) % MAX_16B - MAX_15B

func read_binary_file(file_path: String) -> void:
	var file = FileAccess.open(file_path, FileAccess.READ)
	file.big_endian = true
	var arr: Array[Vector3] = []
	
	var max_x: int = -1000000
	var min_x: int = 1000000
	var max_y: int = -1000000
	var min_y: int = 1000000
	
	while !file.eof_reached():
		var x = unsigned16_to_signed(file.get_16())
		var y = unsigned16_to_signed(file.get_16())
		var flags = file.get_8()
		max_x = maxi(x, max_x)
		max_y = maxi(y, max_y)
		min_x = mini(x, min_x)
		min_y = mini(y, min_y)
		arr.push_back(Vector3(x, y, flags))
		
	file.close()
	
	var size_x = max_x - min_x + 1
	var size_y = max_y - min_y + 1
	map_size = Vector2(size_x, size_y)
	top_left_offset = Vector2(min_x, min_y)
	
	var image = Image.create(size_x, size_y, false, Image.FORMAT_R8)
	
	for tile in arr:
		var x = tile.x - min_x
		var y = size_y - 1 - (tile.y - min_y)
		var flags = tile.z / 255.0
		image.set_pixel(x, y, Color(flags, 0.0, 0.0, 0.0))
		
	var texture = ImageTexture.create_from_image(image)
	map_rect.material.set_shader_parameter("map_data", texture)

	image.save_png("res://src/ui/map/map_data.png")

func _ready():
	read_binary_file("res://src/ui/map/map_data.bin")
	set_zoom(4)
	
var drag_position
var last_mouse_tile: Vector2i

func _on_color_rect_gui_input(event):
	if zoom_enabled and event is InputEventMouseButton:
		if event.button_index == MOUSE_BUTTON_LEFT:
			if event.pressed:
				drag_position = get_global_mouse_position() - map_rect.get_global_rect().position
			else:
				drag_position = null
				var mouse_tile: Vector2i = floor(event.position / float(zoom_value))
				emit_signal("click_on_tile", mouse_tile)
				
		if not event.pressed:
			if event.button_index == MOUSE_BUTTON_WHEEL_UP:
				zoom(1)
			if event.button_index == MOUSE_BUTTON_WHEEL_DOWN:
				zoom(-1)
				
	if event is InputEventMouseMotion:
#		var rel_position = get_global_mouse_position() - map_rect.get_global_rect().position
#		rel_position.x = round(rel_position.x / 2)
#		rel_position.y = round(rel_position.y / 2)
		
		var mouse_tile: Vector2i = floor(event.position / float(zoom_value))
		if last_mouse_tile != mouse_tile:
			map_rect.material.set_shader_parameter("selected_tile", mouse_tile)
			$Label_MousePosition.text = "Mouse " + str(mouse_tile + top_left_offset)
			last_mouse_tile = mouse_tile
		
		if drag_enabled and drag_position:
			var new_pos = get_global_mouse_position() - drag_position
			map_rect.position = new_pos

var zoom_value:int = 4
func zoom(dir: int) -> void:
	var new_zoom_value:int = zoom_value + sign(dir)
	if new_zoom_value >= 1 or new_zoom_value <= 32:
		set_zoom(new_zoom_value)

func set_zoom(new_zoom_value: int) -> void:
	map_rect.position = new_zoom_value * (map_rect.position / float(zoom_value))
#	map_rect.size = new_zoom_value * map_size
	
	zoom_value = new_zoom_value
	
	map_rect.material.set_shader_parameter("size", float(zoom_value))
	map_rect.material.set_shader_parameter("line_width_px", floor(1.0 + float(zoom_value) / 16.0) )

func set_center_position(_parcel_position: Vector2i):
	pass 
	#map_rect.position = -(parcel_position * zoom_value - top_left_offset * zoom_value)
	
