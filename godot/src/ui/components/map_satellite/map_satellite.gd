extends Control

signal clicked_parcel(parcel: Vector2i)


const TILE_SIZE = Vector2(512, 512)
const GRID_SIZE = Vector2(16, 16)
const MAP_SIZE = TILE_SIZE * GRID_SIZE
const PARCELS_PER_TILE = Vector2(20, 20)
const PARCEL_SIZE = TILE_SIZE / PARCELS_PER_TILE
const MAP_CENTER = MAP_SIZE / 2

var dragging := false
var drag_start_mouse := Vector2()
var drag_start_cam_pos := Vector2()
const CLICK_THRESHOLD := 5.0
const MIN_ZOOM := Vector2(0.25, 0.25)
const MAX_ZOOM := Vector2(1.5, 1.5)
var distance

@onready var map_viewport: SubViewport = %MapViewport
@onready var map: Control = %Map
@onready var camera: Camera2D = %Camera2D

const IMAGE_FOLDER = "res://src/ui/components/map_satellite/assets/4/"

func _ready():
	get_viewport().connect("size_changed", self._on_screen_resized)
	update_viewport_size()
	center_camera_on_genesis_plaza()
	# CENTER TO 0,0 (is not in exact position
	
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
			
func _on_screen_resized() -> void:
	update_viewport_size()
	clamp_camera_position()

func update_viewport_size() -> void:
	map_viewport.size = size

func _on_map_gui_input(event: InputEvent) -> void:
	if event is InputEventMouseButton:
		if event.button_index == MOUSE_BUTTON_LEFT:
			if event.pressed:
				dragging = true
				drag_start_mouse = event.position
				drag_start_cam_pos = camera.position
			else:
				distance = drag_start_mouse.distance_to(event.position)
				if distance < CLICK_THRESHOLD:
					var parcel:Vector2i = get_parcel_from_click(event.position)
					print("Clicked parcel: ", parcel)
					emit_signal('clicked_parcel', parcel)
				dragging = false

		elif event.button_index == MOUSE_BUTTON_WHEEL_UP:
			var new_zoom = camera.zoom * 1.1
			camera.zoom = clamp(new_zoom, MIN_ZOOM, MAX_ZOOM)
			
		elif event.button_index == MOUSE_BUTTON_WHEEL_DOWN:
			var new_zoom = camera.zoom * 0.9
			if MAP_SIZE.x * new_zoom.x >= size.x and MAP_SIZE.y * new_zoom.y >= size.y:
				camera.zoom = clamp(new_zoom, MIN_ZOOM, MAX_ZOOM)
			
	elif event is InputEventMouseMotion:
		if dragging:
			var delta = event.position - drag_start_mouse
			camera.position = drag_start_cam_pos - delta * camera.zoom
		
	clamp_camera_position()

func get_parcel_from_click(global_click_pos: Vector2) -> Vector2i:
	var relative_pos = global_click_pos - MAP_CENTER
	var parcel_coord = relative_pos / PARCEL_SIZE
	# TODO: Why I need to plus 7 to get correct coordinates??
	return Vector2i(round_coord(parcel_coord.x)+7, round_coord(parcel_coord.y)+7)

func round_coord(a:float) -> int:
	if a < 0:
		return ceil(a)
	else:
		return floor(a)

func clamp_camera_position() -> void:
	var min_pos := size / 2 / camera.zoom
	var max_pos := MAP_SIZE - min_pos

	# Si el mapa es más chico que la pantalla con el zoom actual, centrar
	if MAP_SIZE.x * camera.zoom.x < size.x:
		camera.position.x = ( size.x  + MAP_SIZE.x ) / 2
	else:
		camera.position.x = clamp(camera.position.x, min_pos.x, max_pos.x)

	if MAP_SIZE.y * camera.zoom.y < size.y:
		camera.position.y = ( size.y  + MAP_SIZE.y ) / 2
	else:
		camera.position.y = clamp(camera.position.y, min_pos.y, max_pos.y)

func center_camera_on_genesis_plaza() -> void:
	camera.position = (TILE_SIZE * GRID_SIZE / 2) - Vector2(180, 190)
	camera.zoom = Vector2(1, 1)

func async_load_pois():
	var url: String = "https://dcl-lists.decentraland.org/pois"
	var headers = {"Content-Type": "application/json"}
	var promise: Promise = Global.http_requester.request_json(
		url, HTTPClient.METHOD_POST, "", headers
	)
	var result = await PromiseUtils.async_awaiter(promise)
	if result is PromiseError:
		printerr("Error request places", result.get_error())
		return
	var json: Dictionary = result.get_string_response_as_json()
	return json	
