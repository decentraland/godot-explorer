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
var tween: Tween

const MAP_PIN := preload("res://src/ui/components/map_satellite/map_pin.tscn")
const PLACE_CARD := preload("res://src/ui/components/map_satellite/place_card.tscn")

const PLACE_CATEGORY_FILTER_BUTTON = preload("res://src/ui/components/map_satellite/place_category_filter_button.tscn")

@onready var margin_container: MarginContainer = $MarginContainer
@onready var panel_button_back: Panel = %PanelButtonBack
@onready var h_box_container_back: HBoxContainer = %HBoxContainerBack
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

const IMAGE_FOLDER = "res://src/ui/components/map_satellite/assets/4/"
var map_is_on_top: bool = false
var filtered_places: Array = []
var is_sidebar_open: bool = false

func _ready():
	if not is_sidebar_open:
		sidebar.position = Vector2(size.x-5, 0)
		h_box_container_back.layout_direction = Control.LAYOUT_DIRECTION_LTR
	else:
		sidebar.position = Vector2(-sidebar.size.x,0 )
		h_box_container_back.layout_direction = Control.LAYOUT_DIRECTION_RTL
		
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
	
	var poi_places = await async_load_category('poi')
	for i in range(poi_places.size()):
		spawn_pin(13, poi_places[i].positions, poi_places[i].title)
		
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
	coordinates_label.label_settings.font_size = FONT_SIZE / camera.zoom.x
	coordinates_label.label_settings.outline_size = OUTLINE_SIZE / camera.zoom.x

func spawn_pin(category:int, coords:Array, title:String):
	var pin = MAP_PIN.instantiate()
	var center_coord:Vector2i
	if coords.size() != 1:
		center_coord = get_center_from_rect_coords(coords)
	else:
		var parts = coords[0].split(",")
		var x = parts[0].to_int()
		var y = -parts[1].to_int()
		center_coord = Vector2i(x,y)
		
	print(title, '- position: ', center_coord)
	var pos = get_parcel_position(center_coord) - pin.size / 2
	pin.pin_category = category
	pin.scene_title = title
	pin.z_index = cursor_marker.z_index+1
	
	pin.position = pos
	map.add_child(pin)

func spawn_card(title:String, contact:Variant, img_source:String):
	var card = PLACE_CARD.instantiate()
	if typeof(contact) == TYPE_STRING:
		card.contact_name = contact
	card.title_place = title
	card.img_url = img_source
	
	cards_v_box_container.add_child(card)

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
		url = "https://places.decentraland.org/api/places/?categories=art&categories=crypto&categories=social&categories=game&categories=shop&categories=education&categories=music&categories=fashion&categories=casino&categories=sports&categories=business&categories=poi"
	else:
		url = "https://places.decentraland.org/api/places/?categories=%s" % category

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
	if not pressed:
		filtered_places = []
		for child in map.get_children():
			if child is MapPin and child.pin_category == type:
				child.queue_free()
		for child in cards_v_box_container.get_children():
			child.queue_free()
			toggle_sidebar_visibility(false)
	else:
		var poi_places = await async_load_category(Place.Categories.keys()[type].to_lower())
		for i in range(poi_places.size()):
			var place = poi_places[i]
			spawn_pin(type, place.positions, place.title)
			spawn_card(place.title, place.contact_name, place.image)
			toggle_sidebar_visibility(true)


func _on_panel_button_back_gui_input(event: InputEvent) -> void:
	if event is InputEventMouseButton:
		if event.button_index == MOUSE_BUTTON_LEFT and event.pressed:
				toggle_sidebar_visibility(!is_sidebar_open)


func toggle_sidebar_visibility(arg:bool)->void:
	var duration = 0.3
	tween = get_tree().create_tween()	
	if not arg:
		is_sidebar_open = false
		tween.tween_property(sidebar, "position", Vector2(size.x-5, 0), duration).set_trans(Tween.TRANS_SINE).set_ease(Tween.EASE_OUT)
		h_box_container_back.layout_direction = Control.LAYOUT_DIRECTION_LTR
	else:
		is_sidebar_open = true
		tween.tween_property(sidebar, "position", Vector2(size.x-sidebar.size.x, 0), duration).set_trans(Tween.TRANS_SINE).set_ease(Tween.EASE_OUT)
		h_box_container_back.layout_direction = Control.LAYOUT_DIRECTION_RTL
