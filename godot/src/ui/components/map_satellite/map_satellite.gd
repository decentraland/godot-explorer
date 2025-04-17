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

const MAP_PIN := preload("res://src/ui/components/map_satellite/map_pin.tscn")
const DISCOVER_CARROUSEL_ITEM = preload("res://src/ui/components/discover/carrousel/discover_carrousel_item.tscn")
const PLACE_CATEGORY_FILTER_BUTTON = preload("res://src/ui/components/map_satellite/place_category_filter_button.tscn")

@onready var margin_container: MarginContainer = $MarginContainer
@onready var cards_v_box_container: VBoxContainer = %CardsVBoxContainer

@onready var cursor_marker: Sprite2D = %CursorMarker
@onready var sub_viewport_container: SubViewportContainer = $SubViewportContainer
@onready var map_viewport: SubViewport = %MapViewport
@onready var map: Control = %Map
@onready var camera: Camera2D = %Camera2D
@onready var coordinates_label: Label = %CoordinatesLabel
@onready var animation_player: AnimationPlayer = $AnimationPlayer
@onready var filter_container: HBoxContainer = %FilterContainer
@onready var sidebar: Control = %Sidebar
@onready var color_rect_close_sidebar: ColorRect = %ColorRectCloseSidebar
@onready var cards_scroll_container: ScrollContainer = %CardsScrollContainer
@onready var map_searchbar: PanelContainer = %MapSearchbar

const IMAGE_FOLDER = "res://src/ui/components/map_satellite/assets/4/"
const SIDE_BAR_WIDTH = 300
var map_is_on_top: bool = false
var filtered_places: Array = []
var active_filter: int = -1
var poi_places = []

func _ready():
	map_searchbar.clean_searchbar.connect(_close_from_searchbar)
	map_searchbar.submited_text.connect(_submitted_text_from_searchbar)
	color_rect_close_sidebar.hide()
	sidebar.position = Vector2(color_rect_close_sidebar.size.x-5, 0)	
	var group := ButtonGroup.new()
	group.allow_unpress = true
	for i in range(13):
		var btn: PlaceFilterButton = PLACE_CATEGORY_FILTER_BUTTON.instantiate()
		btn.button_group = group
		btn.toggle_mode = true
		btn.filter_type = i
		btn.connect("filter_toggled", Callable(self, "_on_filter_button_toggled"))
		filter_container.add_child(btn)
		
	map_viewport.size = size
	
	# Maybe we can remove this line
	get_viewport().connect("size_changed", self._on_screen_resized)

	center_camera_on_genesis_plaza()
	
	
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
	
	poi_places = await async_load_category('poi')
	for i in range(poi_places.size()):
		spawn_pin(13, poi_places[i])
		
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
	show_cursor_at_parcel(parcel_coords)

func get_parcel_position(parcel: Vector2i) -> Vector2:
	var parcel_position = Vector2(parcel + PARCEL_OFFSET) * PARCEL_SIZE + PARCEL_SIZE / 2
	return parcel_position

func show_cursor_at_parcel(parcel: Vector2i):
	var pos = get_parcel_position(parcel)
	cursor_marker.position = pos
	cursor_marker.visible = true
	
	coordinates_label.text = '%s, %s' % [parcel.x, -parcel.y]
	coordinates_label.show()
	update_label_settings()

func update_label_settings() -> void:
	const FONT_SIZE = 18
	const OUTLINE_SIZE = 6
	coordinates_label.label_settings.font_size = int(FONT_SIZE / camera.zoom.x)
	coordinates_label.label_settings.outline_size = int(OUTLINE_SIZE / camera.zoom.x)

func spawn_pin(category:int, place):
	var pin = MAP_PIN.instantiate()
	var center_coord:Vector2i
	if place.positions.size() != 1:
		center_coord = get_center_from_rect_coords(place.positions)
	else:
		var parts = place.positions[0].split(",")
		var x = parts[0].to_int()
		var y = -parts[1].to_int()
		center_coord = Vector2i(x,y)
		
	var pos = get_parcel_position(center_coord) - pin.size / 2
	pin.pin_category = category
	pin.scene_title = place.title
	pin.z_index = cursor_marker.z_index+1
	
	pin.position = pos
	map.add_child(pin)

func create_place_card(place)->void:
	var item = DISCOVER_CARROUSEL_ITEM.instantiate()
	cards_v_box_container.add_child(item)
	item.set_data(place)

func get_center_from_rect_coords(coords: Array) -> Vector2i:
	var min_x = INF
	var max_x = -INF
	var min_y = INF
	var max_y = -INF

	for coord_str in coords:
		var parts = coord_str.split(",")
		if parts.size() != 2:
			continue

		var x = parts[0].to_int()+1
		var y = -parts[1].to_int()

		min_x = min(min_x, x)
		max_x = max(max_x, x)
		min_y = min(min_y, y)
		max_y = max(max_y, y)

	var center_x = floor((min_x + max_x) / 2.0)
	var center_y = floor((min_y + max_y) / -2.0)

	return Vector2i(center_x, -center_y)

func async_load_category(category:String) -> Array:
	var url: String
	if category == 'all':
		url = "https://places.decentraland.org/api/places?offset=0&limit=50&order_by=most_active&order=desc&with_realms_detail=true"
	else:
		url = "https://places.decentraland.org/api/places?offset=0&limit=50&order_by=most_active&order=desc&categories=%s&with_realms_detail=true" % category

	var promise: Promise = Global.http_requester.request_json(url, HTTPClient.METHOD_GET, "", {})
	var result = await PromiseUtils.async_awaiter(promise)

	if result is PromiseError:
		printerr("Error request POIs: ", result.get_error())
		return []

	var json: Dictionary = result.get_string_response_as_json()
	if json.has("data"):
		return json.data
	else:
		return []


func _on_filter_button_toggled(pressed: bool, type: int):
	print('type: ', Place.Categories.keys()[type].to_lower())
	if not pressed:
		filtered_places = []
		for child in map.get_children():
			if child is MapPin and child.pin_category == type:
				child.queue_free()
		for child in cards_v_box_container.get_children():
			child.queue_free()
			map_searchbar.reset()
		_close_sidebar()
	else:
		active_filter = type
		filtered_places = await async_load_category(Place.Categories.keys()[type].to_lower())
		for i in range(filtered_places.size()):
			var place = filtered_places[i]
			if place.title == "Empty":
				continue
			#if place in poi_places:
				#continue
			spawn_pin(type, place)
			create_place_card(place)
		_open_sidebar()
	
		
		map_searchbar.filter_type = type
		map_searchbar.update_filtered_category()
		


func _on_color_rect_gui_input(event: InputEvent) -> void:
	if event is InputEventScreenTouch:
		if !event.pressed:
			_close_sidebar()
			
func _open_sidebar()->void:
		var duration = .4
		create_tween().tween_property(sidebar, "position", Vector2(size.x-sidebar.size.x, 0), duration).set_trans(Tween.TRANS_LINEAR).set_ease(Tween.EASE_OUT)

func _close_sidebar()->void:
		var duration = .4
		create_tween().tween_property(sidebar, "position", Vector2(size.x-5, 0), duration).set_trans(Tween.TRANS_LINEAR).set_ease(Tween.EASE_OUT)
		cards_scroll_container.scroll_vertical = 0

func _close_from_searchbar():
	for child in filter_container.get_children():
		if child is Button and child.toggle_mode and child.filter_type == active_filter:
			child.button_pressed = false
	_close_sidebar()
	
func _submitted_text_from_searchbar(text:String):
	_open_sidebar()
