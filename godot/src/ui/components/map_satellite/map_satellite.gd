extends Control

signal clicked_parcel(parcel: Vector2i)

const TILE_SIZE = Vector2(512, 512)
const GRID_SIZE = Vector2(16, 16)
const PARCELS_PER_TILE = Vector2(20, 20)
const PARCEL_SIZE = TILE_SIZE / PARCELS_PER_TILE
# to generate a map with boundaries in -170,-170 and 170, 170
const MAP_SIZE = PARCEL_SIZE * Vector2(340,340)
const PARCEL_OFFSET = Vector2i(170,170)
const MAP_CENTER = MAP_SIZE / 2
const TILE_DISPLACEMENT = Vector2(18,18) * PARCEL_SIZE
const MIN_ZOOM := Vector2(1, 1)
const MAX_ZOOM := Vector2(2, 2)
const MAP_MARKER = preload("res://src/ui/components/map_satellite/map_marker.tscn")
const MAP_PIN := preload("res://src/ui/components/map_satellite/map_pin.tscn")
const DISCOVER_CARROUSEL_ITEM = preload("res://src/ui/components/discover/carrousel/discover_carrousel_item.tscn")
const PLACE_CATEGORY_FILTER_BUTTON = preload("res://src/ui/components/map_satellite/place_category_filter_button.tscn")
const ARCHIPELAGO_CIRCLE = preload("res://src/ui/components/map_satellite/archipelago_circle.tscn")
var active_touches := {}
var last_pinch_distance := 0.0
var last_pan_position := Vector2.ZERO
var PAN_THRESHOLD := 5.0
var pan_started := false
var just_zoomed := false 

var dragging:=false
var last_mouse_position: Vector2

@onready var archipelagos_control: Control = %ArchipelagosControl
@onready var sub_viewport_container: SubViewportContainer = $SubViewportContainer
@onready var map_viewport: SubViewport = %MapViewport
@onready var map: Control = %Map
@onready var camera: Camera2D = %Camera2D

# Searchbar and Filters
@onready var searchbar: PanelContainer = %Searchbar
@onready var archipelago_button: Button = %ArchipelagoButton
@onready var h_box_container_filters: HBoxContainer = %HBoxContainer_Filters

# Cards, filter result
@onready var no_results: VBoxContainer = %NoResults
@onready var cards: BoxContainer = %Cards
@onready var cards_scroll: ScrollContainer = %CardsScroll
@onready var sidebar_container: BoxContainer = %SidebarContainer


const IMAGE_FOLDER = "res://src/ui/components/map_satellite/assets/4/"
const SIDE_BAR_WIDTH = 300
var map_is_on_top: bool = false
var filtered_places: Array = []
var active_filter: int = -1
var poi_places_ids = []
var live_places_ids = []
var show_poi:= true
var show_live:= true

func _ready():
	if not map.is_connected("map_tapped", Callable(self, "handle_tap")):
		map.connect("map_tapped", Callable(self, "handle_tap"))
	UiSounds.install_audio_recusirve(self)
	_close_sidebar()
	
	get_viewport().connect("size_changed", self._on_viewport_resized)
	archipelago_button.toggle_mode = true
	archipelagos_control.visible = archipelago_button.button_pressed
	searchbar.clean_searchbar.connect(_close_from_searchbar)
	searchbar.submited_text.connect(_submitted_text_from_searchbar)
	searchbar.reset()
	var group := ButtonGroup.new()
	group.allow_unpress = true
	for i in range(13):
		var btn: PlaceFilterButton = PLACE_CATEGORY_FILTER_BUTTON.instantiate()
		btn.button_group = group
		btn.toggle_mode = true
		btn.filter_type = i
		btn.connect("filter_toggled", Callable(self, "_on_filter_button_toggled"))
		h_box_container_filters.add_child(btn)
	map_viewport.size = MAP_SIZE
	map.size = MAP_SIZE
	center_camera_on_parcel(Vector2i(0,1))
	
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
				tex_rect.position = Vector2(x * TILE_SIZE.x, y * TILE_SIZE.y) + TILE_DISPLACEMENT
				map.add_child(tex_rect)
			else:
				push_error("Error loading map image: " + image_path)
	
		
	var poi_places = await async_load_category('poi')
	poi_places_ids = poi_places.map(func(poi_place): return poi_place.id )
	for i in range(poi_places.size()):
		spawn_pin(13, poi_places[i], 'poi_pins')
		
	var live_places = await async_load_category('live')
	live_places_ids = live_places.map(func(live_place): return live_place.id )
	for i in range(live_places.size()):
		spawn_pin(14, live_places[i], 'live_pins')	
		
	_async_draw_archipelagos()
	#var circle = ARCHIPELAGO_CIRCLE.instantiate()

func _on_viewport_resized()->void:
	map_viewport.size = size
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

func center_camera_on_parcel(parcel:Vector2i) -> void:
	var target_position = get_parcel_position(parcel)
	var tween = create_tween()
	tween.tween_property(camera, "position", target_position, 0.3).set_trans(Tween.TRANS_LINEAR).set_ease(Tween.EASE_OUT)
	await tween.finished

func get_parcel_position(parcel: Vector2i) -> Vector2:
	var parcel_position = Vector2(parcel + PARCEL_OFFSET) * PARCEL_SIZE + PARCEL_SIZE / 2
	return parcel_position

func show_marker_at_parcel(parcel: Vector2i):
	prints("Show market at parcel", parcel)
	for child in map.get_children():
		if child is Marker:
			child.queue_free()
			
	center_camera_on_parcel(parcel)
	var marker = MAP_MARKER.instantiate()
	var pos = get_parcel_position(parcel) - marker.size / 2
	marker.position = pos	
	marker.marker_x = parcel.x
	marker.marker_y = -parcel.y
	map.add_child(marker)
	marker.visible = true
	marker.update()

func spawn_pin(category:int, place, group:String):
	var pin = MAP_PIN.instantiate()
	var center_coord:Vector2i
	if category != 14:
		pin.scene_title = place.title
		if place.positions.size() != 1:
			center_coord = get_center_from_rect_coords(place.positions)
		else:
			var parts = place.positions[0].split(",")
			var x = parts[0].to_int()
			var y = -parts[1].to_int()
			center_coord = Vector2i(x,y)
	else:
		center_coord = Vector2i(place.x, -place.y)
		pin.scene_title = place.name
		
	var pos = get_parcel_position(center_coord) - pin.size / 2
	pos.y -= pin.size.y / 2 - 8
	
	pin.pin_x = center_coord.x
	pin.pin_y = center_coord.y
	pin.touched_pin.connect(self._on_pin_clicked)

	pin.pin_category = category
	pin.z_index = 5
	
	pin.position = pos
	map.add_child(pin)
	pin.add_to_group(group)
	if group == 'hidden':
		pin.visible = false
		
func _on_pin_clicked(coord:Vector2i):
	clicked_parcel.emit(coord)
	show_marker_at_parcel(coord)

func create_place_card(place)->void:
	var card = DISCOVER_CARROUSEL_ITEM.instantiate()
	card.item_pressed.connect(_item_pressed)
	cards.add_child(card)
	card.set_data(place)

func _item_pressed(place)->void:
	show_marker_at_parcel(get_center_from_rect_coords(place.positions))

func get_center_from_rect_coords(coords: Array) -> Vector2i:
	if coords.size() == 1:
		var parts = coords[0].split(",")
		var x = parts[0].to_int()
		var y = -parts[1].to_int()
		return Vector2i(x, y)
		
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

func async_load_text_search(value: String) -> Array:
	var url = "https://places.decentraland.org/api/places?search=%s&offset=0&limit=50&order_by=most_active&order=desc&with_realms_detail=true" % value
	var promise: Promise = Global.http_requester.request_json(url, HTTPClient.METHOD_GET, "", {})
	var result = await PromiseUtils.async_awaiter(promise)
	if result is PromiseError:
		printerr("Error searching places: ", result.get_error())
		return []

	var json: Dictionary = result.get_string_response_as_json()
	if json.has("data"):
		return json.data
	else:
		return []

func async_load_category(category:String) -> Array:
	var url: String
	if category == 'all':
		url = "https://places.decentraland.org/api/places?offset=0&limit=50&order_by=most_active&order=desc&with_realms_detail=true"
	elif category == 'live':
		url = "https://events.decentraland.org/api/events/?list=live"
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
	var places_to_show = 0
	_clean_list_and_pins()
	if not pressed:
		filtered_places = []
		searchbar.reset()
		_close_sidebar(0.4)
	else:
		
		active_filter = type
		filtered_places = await async_load_category(Place.Categories.keys()[type].to_lower())
		
			
		for i in range(filtered_places.size()):
			var place = filtered_places[i]
			if place.title == "Empty":
				continue
			create_place_card(place)
			places_to_show = places_to_show + 1
			if place.id in poi_places_ids:
				spawn_pin(type, place, 'hidden')
			else:
				spawn_pin(type, place, 'pins')
			
		if places_to_show == 0:
			no_results.show()
			cards_scroll.hide()
		else:
			no_results.hide()
			cards_scroll.show()
		_open_sidebar()
	
		
		searchbar.filter_type = type
		searchbar.update_filtered_category()
		

func _open_sidebar()->void:
	sidebar_container.show()
	#var duration = .4
	#var tween = create_tween()
	#tween.tween_property(sidebar_container, "modulate", 1, duration).set_trans(Tween.TRANS_LINEAR).set_ease(Tween.EASE_OUT)

func _close_sidebar(duration:float=0.0)->void:
	#var tween = create_tween()
	#tween.tween_property(sidebar_container, "modulate", 0, duration).set_trans(Tween.TRANS_LINEAR).set_ease(Tween.EASE_OUT)
	cards_scroll.scroll_horizontal = 0
	cards_scroll.scroll_vertical = 0
	filtered_places = []
	sidebar_container.hide()
	
func _close_from_searchbar():
	for child in h_box_container_filters.get_children():
		if child is Button and child.toggle_mode and child.filter_type == active_filter:
			child.button_pressed = false
	_clean_list_and_pins()
	_close_sidebar()

func _submitted_text_from_searchbar(text:String):
	var places_to_show = 0
	_clean_list_and_pins()
	filtered_places = await async_load_text_search(text)
	for i in range(filtered_places.size()):
			var place = filtered_places[i]
			if place.title != "Empty":
				create_place_card(place)
				places_to_show = places_to_show + 1
			if place.id in poi_places_ids:
				spawn_pin(0, place, 'hidden')
			spawn_pin(0, place, 'pins')
	if places_to_show == 0:
		no_results.show()
		cards_scroll.hide()
	else:
		no_results.hide()
		cards_scroll.show()
	_open_sidebar()

func _clean_list_and_pins()->void:
	for child in cards.get_children():
		child.queue_free()
	for child in map.get_children():
		if child.is_in_group('pins') or child.is_in_group('hidden'):
			child.queue_free() 

func _async_draw_archipelagos() -> void:
	const URL = "https://archipelago-ea-stats.decentraland.org/hot-scenes"
	var promise: Promise = Global.http_requester.request_json(URL, HTTPClient.METHOD_GET, "", {})
	var result = await PromiseUtils.async_awaiter(promise)

	var archipelagos_array = []
	if result is PromiseError:
		printerr("Error requesting archipelagos: ", result.get_error())
	else:
		archipelagos_array = result.get_string_response_as_json()
		for archipelago in archipelagos_array:
			var circle = ARCHIPELAGO_CIRCLE.instantiate()
			var center_coord
			if archipelago.parcels.size() != 1:
				center_coord = _get_center_from_rect_coords_array(archipelago.parcels)
			else:
				var x = archipelago.parcels[0][0]
				var y = -archipelago.parcels[0][1]
				center_coord = Vector2i(x,-y)
			var radius = 50 + archipelago.usersTotalCount * 10
			var pos = get_parcel_position(center_coord)
			archipelagos_control.add_child(circle)
			circle.set_circle(pos, radius)

func _get_center_from_rect_coords_array(coords: Array) -> Vector2i:
	var min_x = INF
	var max_x = -INF
	var min_y = INF
	var max_y = -INF

	for coord_array in coords:
		var x = coord_array[0]
		var y = -coord_array[1]

		min_x = min(min_x, x)
		max_x = max(max_x, x)
		min_y = min(min_y, y)
		max_y = max(max_y, y)

	var center_x = floor((min_x + max_x) / 2.0)
	var center_y = floor((min_y + max_y) / -2.0)

	return Vector2i(center_x, -center_y)


func handle_tap(pos: Vector2):
	var coords = pos / PARCEL_SIZE
	var parcel_coords = Vector2i(coords) - PARCEL_OFFSET
	clicked_parcel.emit(parcel_coords)
	show_marker_at_parcel(parcel_coords)

func _on_archipelago_button_toggled(toggled_on: bool) -> void:
	archipelagos_control.visible = toggled_on

func _on_show_poi_toggled(toggled_on: bool) -> void:
	show_poi = toggled_on
	for child in map.get_children():
		if child.is_in_group('poi_pins'):
			child.visible = toggled_on
		if child.is_in_group('hidden'):
			child.visible = !toggled_on
			
func _on_show_live_toggled(toggled_on: bool) -> void:
	show_live = toggled_on
	for child in map.get_children():
		if child.is_in_group('live_pins'):
			child.visible = toggled_on
		
