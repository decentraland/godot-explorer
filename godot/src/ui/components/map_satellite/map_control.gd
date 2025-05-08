extends Control

signal on_move
signal parcel_click(parcel_position: Vector2i)

@export var drag_enabled: bool = true
@export var zoom_value: int = 20
@onready var map: Control = %Map

const TILE_SIZE = Vector2(512, 512)
const GRID_SIZE = Vector2(16, 16)
const PARCELS_PER_TILE = Vector2(20, 20)
const PARCEL_SIZE = TILE_SIZE / PARCELS_PER_TILE
const IMAGE_FOLDER = "res://src/ui/components/map_satellite/assets/4/"
const TILE_DISPLACEMENT = Vector2(18,18) * PARCEL_SIZE

# Draging variables
var is_dragging = false
var dirty_is_dragging = false
var drag_position: Vector2
var start_dragging_position: Vector2

# The size of the map in parcels
var map_parcel_size: Vector2
# The top left parcel
var map_topleft_parcel_position: Vector2


func _ready():
	map_parcel_size = Vector2(512, 512)
	map_topleft_parcel_position = Vector2(-256, -256)

	# TODO: use "https://api.decentraland.org/v1/minimap.png"

	set_zoom(zoom_value)
	set_center_position(Vector2(0, 0))
	for y in range(GRID_SIZE.y):
		for x in range(GRID_SIZE.x):
			var image_path = IMAGE_FOLDER + "%d,%d.jpg" % [x, y]
			var tex = load(image_path) as Texture2D
			if tex:
				var tex_rect = TextureRect.new()
				tex_rect.texture = tex
				tex_rect.stretch_mode = TextureRect.STRETCH_SCALE
				tex_rect.size = TILE_SIZE
				tex_rect.position = Vector2(x * TILE_SIZE.x, y * TILE_SIZE.y) + TILE_DISPLACEMENT
				map.add_child(tex_rect)
			else:
				push_error("Error loading map image: " + image_path)

func set_zoom(new_zoom_value: int) -> void:
	var center_parcel_position: Vector2 = get_center_position()
	zoom_value = new_zoom_value
	map.size = map_parcel_size * new_zoom_value

	set_center_position(center_parcel_position)
	#_update_used_parcels()


func set_center_position(parcel_position: Vector2):
	var absolute_position = (
		Vector2(parcel_position.x, -parcel_position.y) - map_topleft_parcel_position
	)
	var position_in_color_rect = Vector2(absolute_position) * zoom_value
	var new_position = -position_in_color_rect + self.size / 2.0 - (Vector2.ONE * zoom_value / 2.0)
	map.position = new_position


func get_center_position() -> Vector2:
	var position_in_color_rect: Vector2 = -(
		map.position - self.size / 2.0 + (Vector2.ONE * zoom_value / 2.0)
	)
	var absolute_position: Vector2 = Vector2(position_in_color_rect / zoom_value)
	var inverted_parcel_position: Vector2 = absolute_position + map_topleft_parcel_position
	return Vector2(inverted_parcel_position.x, -inverted_parcel_position.y)


func _on_map_gui_input(event):
	if event is InputEventMouseButton:
		if event.button_index == MOUSE_BUTTON_LEFT:
			if event.pressed:
				dirty_is_dragging = true
				drag_position = get_global_mouse_position() - map.position
				start_dragging_position = get_global_mouse_position()
				self.reflect_dragging.call_deferred()
			else:
				dirty_is_dragging = false
				self.reflect_dragging.call_deferred()
				var diff: Vector2 = (
					(get_global_mouse_position() - start_dragging_position) / zoom_value
				)
				if diff.length() < 1:
					parcel_click.emit(get_parcel_from_mouse())

	if event is InputEventMouseMotion:
		if drag_enabled and dirty_is_dragging:
			var new_pos = get_global_mouse_position() - drag_position
			map.position = new_pos
			on_move.emit()


func reflect_dragging():
	is_dragging = dirty_is_dragging


func get_parcel_from_mouse_real() -> Vector2:
	var absolute_position: Vector2 = Vector2(map.get_local_mouse_position() / zoom_value)
	var position_without_offset: Vector2 = absolute_position + map_topleft_parcel_position
	return Vector2(position_without_offset.x, -position_without_offset.y)


func get_parcel_from_mouse() -> Vector2i:
	var real_position = get_parcel_from_mouse_real()
	return Vector2i(floor(real_position.x), floor(real_position.y))


func set_selected_parcel(parcel_position: Vector2):
	pass
	


func set_used_parcels(used_parcel, emtpy_parcels):
	var total = used_parcel.size() + emtpy_parcels.size()
	var to_delete = map.get_child_count() - total

	if to_delete > 0:
		for i in range(to_delete):
			# TODO: warn: check if get_child(0) is properly set
			var child = map.get_child(0)
			map.remove_child(child)
			child.queue_free()
	elif to_delete < 0:
		for i in range(-to_delete):
			var new_child: ColorRect = ColorRect.new()
			new_child.mouse_filter = Control.MOUSE_FILTER_PASS
			map.add_child(new_child)

	var index: int = 0
	for i in range(used_parcel.size()):
		var color_rect: ColorRect = map.get_child(index)
		color_rect.set_meta("parcel", Vector2(used_parcel[i].x, used_parcel[i].y))
		color_rect.color = Color.DARK_GREEN
		color_rect.color.a = 0.5
		index += 1

	for i in range(emtpy_parcels.size()):
		var color_rect: ColorRect = map.get_child(index)
		color_rect.set_meta("parcel", Vector2(emtpy_parcels[i].x, emtpy_parcels[i].y))
		color_rect.color = Color.ORANGE_RED
		color_rect.color.a = 0.5
		index += 1

	_update_used_parcels()


func _update_used_parcels():
	for child in map.get_children():
		var texture_rect: TextureRect = child

		var parcel_position = Vector2(texture_rect.get_meta("parcel"))
		var inverted_position = Vector2(parcel_position.x, -parcel_position.y)
		var parcel_in_control = inverted_position - map_topleft_parcel_position

		texture_rect.size = Vector2(zoom_value, zoom_value)
		texture_rect.position = zoom_value * (parcel_in_control + Vector2.UP)
