extends Control

signal clicked_parcel(parcel: Vector2i)


const TILE_SIZE = Vector2(512, 512)
const GRID_SIZE = Vector2(16, 16)
const MAP_SIZE = TILE_SIZE * GRID_SIZE
const PARCELS_PER_TILE = Vector2(20, 20)
const PARCEL_SIZE = TILE_SIZE / PARCELS_PER_TILE
const PARCEL_OFFSET = Vector2i(152,152)
const MAP_CENTER = MAP_SIZE / 2

var dragging := false
var drag_start_mouse := Vector2()
var drag_start_cam_pos := Vector2()
const CLICK_THRESHOLD := 5.0
const MIN_ZOOM := Vector2(0.5, 0.5)
const MAX_ZOOM := Vector2(1.5, 1.5)
var distance

var popup_scene := preload("res://src/ui/components/map_satellite/map_popup.tscn")
var popup_instance: Control

@onready var margin_container: MarginContainer = $MarginContainer

@onready var cursor_marker: Sprite2D = %CursorMarker
@onready var sub_viewport_container: SubViewportContainer = $SubViewportContainer
@onready var map_viewport: SubViewport = %MapViewport
@onready var map: Control = %Map
@onready var camera: Camera2D = %Camera2D
@onready var coordinates_label: Label = %CoordinatesLabel
@onready var animation_player: AnimationPlayer = $AnimationPlayer

const IMAGE_FOLDER = "res://src/ui/components/map_satellite/assets/4/"
var map_is_on_top: bool = false

func _ready():
	map_viewport.size = size
	
	# Maybe we can remove this line
	get_viewport().connect("size_changed", self._on_screen_resized)

	center_camera_on_genesis_plaza()
	popup_instance = popup_scene.instantiate()
	popup_instance.hide()
	#sub_viewport_container.add_child(popup_instance)
	sub_viewport_container.add_child(popup_instance)
	
	
	# Drawing the entire map using tiles
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

# Maybe we can remove this function (the viewport isn't resizable) 
func _on_screen_resized() -> void:
	map_viewport.size = size
	clamp_camera_position()

func _on_map_gui_input(event: InputEvent) -> void:
	if event is InputEventMouseButton:
		if event.button_index == MOUSE_BUTTON_LEFT:
			if event.pressed:
				dragging = true
				drag_start_mouse = event.position
				drag_start_cam_pos = camera.position
				popup_instance.hide()
			else:
				distance = drag_start_mouse.distance_to(event.position)
				if distance < CLICK_THRESHOLD:
					handle_click(event.position)
				dragging = false

		elif event.button_index == MOUSE_BUTTON_WHEEL_UP:
			var new_zoom = camera.zoom * 1.1
			camera.zoom = clamp(new_zoom, MIN_ZOOM, MAX_ZOOM)

		elif event.button_index == MOUSE_BUTTON_WHEEL_DOWN:
			var new_zoom = camera.zoom * 0.9
			if MAP_SIZE.x * new_zoom.x >= size.x and MAP_SIZE.y * new_zoom.y >= size.y:
				camera.zoom = clamp(new_zoom, MIN_ZOOM, MAX_ZOOM)

		update_label_settings()
		
	elif event is InputEventMouseMotion:
		if dragging:
			var delta = event.position - drag_start_mouse
			camera.position = drag_start_cam_pos - delta * camera.zoom
		
	clamp_camera_position()

func clamp_camera_position() -> void:
	const MAP_TOP_MARGIN = 50
	var min_pos := size / 2 / camera.zoom
	var max_pos := MAP_SIZE - min_pos

	if MAP_SIZE.x * camera.zoom.x < size.x:
		camera.position.x = ( size.x  + MAP_SIZE.x ) / 2
	else:
		camera.position.x = clamp(camera.position.x, min_pos.x, max_pos.x)

	if MAP_SIZE.y * camera.zoom.y < size.y:
		camera.position.y = ( size.y  + MAP_SIZE.y ) / 2
	else:
		camera.position.y = clamp(camera.position.y, min_pos.y, max_pos.y)
		
	map_is_on_top = camera.position.y <= min_pos.y + MAP_TOP_MARGIN

func _process(_delta):
	if map_is_on_top and margin_container.visible:
		animation_player.play('hide_filters')
	elif not map_is_on_top and not margin_container.visible:
		animation_player.play('show_filters')

func center_camera_on_genesis_plaza() -> void:
	camera.position = (TILE_SIZE * GRID_SIZE / 2) - Vector2(180, 190)
	camera.zoom = Vector2(1, 1)

func handle_click(event_position:Vector2)-> void:
	var coords = event_position / PARCEL_SIZE
	var parcel_coords = Vector2i(coords) - PARCEL_OFFSET
	clicked_parcel.emit(parcel_coords)
	#var msg = str(parcel_coords.x) + ',' + str(parcel_coords.y)
	#popup_instance.set_text(msg)
	#popup_instance.show_at(map_viewport.size)
	show_cursor_at_parcel(parcel_coords)
	
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
	

func get_parcel_position(parcel: Vector2i) -> Vector2:
	var parcel_position = Vector2(parcel + PARCEL_OFFSET) * PARCEL_SIZE + PARCEL_SIZE / 2
	return parcel_position

func show_cursor_at_parcel(parcel: Vector2i):
	var pos = get_parcel_position(parcel)
	cursor_marker.position = pos
	cursor_marker.visible = true
	
	coordinates_label.text = '%s, %s' % [parcel.x, parcel.y]
	coordinates_label.show()
	update_label_settings()

func update_label_settings() -> void:
	const FONT_SIZE = 18
	const OUTLINE_SIZE = 6
	coordinates_label.label_settings.font_size = FONT_SIZE / camera.zoom.x
	coordinates_label.label_settings.outline_size = OUTLINE_SIZE / camera.zoom.x
