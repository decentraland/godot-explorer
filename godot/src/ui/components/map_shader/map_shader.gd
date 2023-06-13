extends Control

signal on_move()
			
@onready var color_rect_map = %ColorRect_Map
@onready var sub_viewport = $SubViewportContainer_MapShader/SubViewport

@export var drag_enabled: bool = true
@export var zoom_value:int = 20

# Draging variables
var is_dragging = false
var _is_dragging = false
var drag_position: Vector2

# The size of the map in parcels
var map_parcel_size: Vector2
# The top left parcel
var map_topleft_parcel_position: Vector2

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
	map_parcel_size = Vector2(size_x, size_y)
	map_topleft_parcel_position = Vector2(min_x, -max_y)
	
	var image = Image.create(size_x, size_y, false, Image.FORMAT_R8)
	
	for tile in arr:
		var x = tile.x - min_x
		var y = size_y - 1 - (tile.y - min_y)
		var flags = tile.z / 255.0
		image.set_pixel(x, y, Color(flags, 0.0, 0.0, 0.0))
		
	var texture = ImageTexture.create_from_image(image)
	color_rect_map.material.set_shader_parameter("map_data", texture)

	image.save_png("res://src/ui/components/map_shader/map_data.png")

func _ready():
	color_rect_map.material = color_rect_map.material.duplicate()
	read_binary_file("res://src/ui/components/map_shader/map_data.bin")
	_on_resized()
	set_zoom(zoom_value)
	set_center_position(Vector2(0,0))
	
func set_zoom(new_zoom_value: int) -> void:
	var center_parcel_position: Vector2 = get_center_position()
	zoom_value = new_zoom_value
	color_rect_map.size = map_parcel_size * new_zoom_value
	color_rect_map.material.set_shader_parameter("size", float(zoom_value))
	color_rect_map.material.set_shader_parameter("line_width_px", floor(1.0 + float(zoom_value) / 16.0) )
	set_center_position(center_parcel_position)
	
func set_center_position(parcel_position: Vector2):
	var absolute_position = Vector2(parcel_position.x, -parcel_position.y) - map_topleft_parcel_position
	var position_in_color_rect = Vector2(absolute_position) * zoom_value
	var new_position = -position_in_color_rect + self.size / 2.0 - (Vector2.ONE * zoom_value / 2.0)
	color_rect_map.position = new_position

func get_center_position() -> Vector2:
	var position_in_color_rect: Vector2 = -(color_rect_map.position - self.size / 2.0 + (Vector2.ONE * zoom_value / 2.0))
	var absolute_position: Vector2 = Vector2(position_in_color_rect / zoom_value)
	var inverted_parcel_position: Vector2 = absolute_position + map_topleft_parcel_position
	return Vector2(inverted_parcel_position.x, -inverted_parcel_position.y)
	
func to_parcel_position(tile:Vector2):
	return Vector2(tile.x + map_topleft_parcel_position.x, -(tile.y + map_topleft_parcel_position.y))

func _on_color_rect_map_gui_input(event):
	if event is InputEventMouseButton:
		if event.button_index == MOUSE_BUTTON_LEFT:
			if event.pressed:
				_is_dragging = true
				drag_position = get_global_mouse_position() - color_rect_map.get_global_rect().position
				self.reflect_dragging.call_deferred()
			else:
				_is_dragging = false
				self.reflect_dragging.call_deferred()
				
	if event is InputEventMouseMotion:
		if drag_enabled and _is_dragging:
			var new_pos = get_global_mouse_position() - drag_position
			color_rect_map.position = new_pos
			emit_signal("on_move")
			
func reflect_dragging():
	is_dragging = _is_dragging
	
func get_parcel_from_mouse_real() -> Vector2:
	var absolute_position: Vector2 = Vector2(color_rect_map.get_local_mouse_position() / zoom_value)
	var inverted_parcel_position: Vector2 = absolute_position + map_topleft_parcel_position
	var ret = Vector2(inverted_parcel_position.x, 1-inverted_parcel_position.y)
	return ret
	
func get_parcel_from_mouse() -> Vector2i:
	var absolute_position: Vector2 = Vector2(color_rect_map.get_local_mouse_position() / zoom_value)
	var inverted_parcel_position: Vector2 = absolute_position + map_topleft_parcel_position
	var ret = Vector2(inverted_parcel_position.x, 1-inverted_parcel_position.y)
	return Vector2i(floor(ret.x), floor(ret.y))
	
func set_selected_parcel(parcel_position: Vector2):
	var color_rect_position = Vector2(parcel_position.x, -parcel_position.y) - map_topleft_parcel_position
	color_rect_map.material.set_shader_parameter("selected_tile", color_rect_position)

func _on_resized():
	if is_instance_valid(sub_viewport):
		sub_viewport.size = self.size
