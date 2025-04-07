extends Control

const TILE_SIZE = Vector2(512, 512)
const GRID_SIZE = Vector2(16, 16)
const MAP_SIZE = TILE_SIZE * GRID_SIZE
const PARCELS_PER_TILE = Vector2(20, 20)
const PARCEL_SIZE = TILE_SIZE / PARCELS_PER_TILE
const MAP_CENTER = MAP_SIZE / 2

var dragging := false
var drag_origin := Vector2()
var map_initial_pos := Vector2()


var zoom := 1.0
var min_zoom := 0.25
var max_zoom := 1.5

@onready var map = $Map

const IMAGE_FOLDER = "res://src/ui/components/map_satellite/assets/4/"

func _ready():
	for y in range(GRID_SIZE.y):
		for x in range(GRID_SIZE.x):
			var image_path = IMAGE_FOLDER + "%d,%d.jpg" % [x, y]
			var tex = load(image_path) as Texture2D
			if tex:
				var tex_rect = TextureRect.new()
				tex_rect.texture = tex
				tex_rect.stretch_mode = TextureRect.STRETCH_SCALE
				tex_rect.size = TILE_SIZE
				tex_rect.position = Vector2(x * TILE_SIZE.x, y * TILE_SIZE.y)
				map.add_child(tex_rect)
			else:
				push_error("Error loading map image: " + image_path)
			
func _on_gui_input(event: InputEvent) -> void:
	if event is InputEventMouseButton:
		if event.button_index == MOUSE_BUTTON_LEFT and event.pressed:
			dragging = true
			drag_origin = event.position
			map_initial_pos = map.position
		elif event.button_index == MOUSE_BUTTON_LEFT and not event.pressed:
			dragging = false

		elif event.button_index == MOUSE_BUTTON_WHEEL_UP:
			zoom = clamp(zoom + 0.1, min_zoom, max_zoom)
			apply_zoom(event.position)
		elif event.button_index == MOUSE_BUTTON_WHEEL_DOWN:
			zoom = clamp(zoom - 0.1, min_zoom, max_zoom)
			apply_zoom(event.position)

	elif event is InputEventMouseMotion and dragging:
		var delta = event.position - drag_origin
		var new_pos = map_initial_pos + delta
		map.position = clamp_map_position(new_pos)
		#map.position = new_pos

func clamp_map_position(pos: Vector2) -> Vector2:
	var visible_rect = get_viewport_rect().size
	var scaled_map_size = MAP_SIZE * map.scale

	var min_x = visible_rect.x - scaled_map_size.x
	var min_y = visible_rect.y - scaled_map_size.y
	var max_x = 0.0
	var max_y = 0.0

	return Vector2(
		clamp(pos.x, min_x, max_x),
		clamp(pos.y, min_y, max_y)
	)
	
func apply_zoom(_focus_point: Vector2):
	map.scale = Vector2(zoom, zoom)
	map.position = clamp_map_position(map.size/2)


func _on_map_gui_input(event: InputEvent) -> void:
	if event is InputEventMouseButton:
		if event.button_index == MOUSE_BUTTON_LEFT and event.pressed and not dragging:
			var parcel = get_parcel_from_click(event.position)
			print("Clicked parcel: ", parcel)

func get_parcel_from_click(global_click_pos: Vector2) -> Vector2i:
	var relative_pos = global_click_pos - MAP_CENTER
	var parcel_coord = relative_pos / PARCEL_SIZE
	# TODO: Why I need to plus 7 to get correct coordinates??
	return Vector2i(round_coord(parcel_coord.x)+7, round_coord(parcel_coord.y)+7)

func round_coord(a:float):
	if a < 0:
		return ceil(a)
	else:
		return floor(a)
