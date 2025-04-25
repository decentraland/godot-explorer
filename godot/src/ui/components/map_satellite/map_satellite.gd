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


const MIN_ZOOM := Vector2(0.2, 0.2)
const MAX_ZOOM := Vector2(2, 2)
var active_touches := {}
var last_pinch_distance := 0.0
var last_pan_position := Vector2.ZERO
var PAN_THRESHOLD := 5.0
var pan_started := false
var just_zoomed := false 

const MAP_PIN := preload("res://src/ui/components/map_satellite/map_pin.tscn")
const DISCOVER_CARROUSEL_ITEM = preload("res://src/ui/components/discover/carrousel/discover_carrousel_item.tscn")
const PLACE_CATEGORY_FILTER_BUTTON = preload("res://src/ui/components/map_satellite/place_category_filter_button.tscn")
const ARCHIPELAGO_CIRCLE = preload("res://src/ui/components/map_satellite/archipelago_circle.tscn")

@onready var margin_container: MarginContainer = $MarginContainer
@onready var cards_v_box_container: VBoxContainer = %CardsVBoxContainer
@onready var cards_scroll_container: ScrollContainer = %CardsScrollContainer
@onready var no_results: VBoxContainer = %NoResults

@onready var archipelagos_control: Control = %ArchipelagosControl

@onready var cursor_marker: Sprite2D = %CursorMarker
@onready var sub_viewport_container: SubViewportContainer = $SubViewportContainer
@onready var map_viewport: SubViewport = %MapViewport
@onready var map: Control = %Map
@onready var camera: Camera2D = %Camera2D
@onready var coordinates_label: Label = %CoordinatesLabel
@onready var animation_player: AnimationPlayer = $AnimationPlayer
@onready var sidebar: Control = %SidebarLandscape
@onready var color_rect_close_sidebar: ColorRect = %ColorRectCloseSidebar
@onready var portrait: VBoxContainer = %Portrait
@onready var landscape: VBoxContainer = %Landscape
@onready var portrait_filters: HBoxContainer = %PortraitFilters
@onready var landscape_filters: HBoxContainer = %LandscapeFilters
@onready var portrait_map_searchbar: PanelContainer = %PortraitMapSearchbar
@onready var landscape_map_searchbar: PanelContainer = %LandscapeMapSearchbar
@onready var archipelago_button: Button = %ArchipelagoButton

const IMAGE_FOLDER = "res://src/ui/components/map_satellite/assets/4/"
const SIDE_BAR_WIDTH = 300
var map_is_on_top: bool = false
var filtered_places: Array = []
var active_filter: int = -1
var poi_places_ids = []
var live_places_ids = []



func _ready():
	get_viewport().connect("size_changed", self._on_viewport_resized)

	archipelagos_control.visible = archipelago_button.button_pressed
	portrait_map_searchbar.clean_searchbar.connect(_close_from_searchbar)
	portrait_map_searchbar.submited_text.connect(_submitted_text_from_searchbar)
	portrait_map_searchbar.reset()
	landscape_map_searchbar.clean_searchbar.connect(_close_from_searchbar)
	landscape_map_searchbar.submited_text.connect(_submitted_text_from_searchbar)
	landscape_map_searchbar.reset()
	color_rect_close_sidebar.hide()
	sidebar.position = Vector2(color_rect_close_sidebar.size.x-5, 0)	
	var group := ButtonGroup.new()
	group.allow_unpress = true
	for i in range(13):
		var btnp: PlaceFilterButton = PLACE_CATEGORY_FILTER_BUTTON.instantiate()
		btnp.button_group = group
		btnp.toggle_mode = true
		btnp.filter_type = i
		btnp.connect("filter_toggled", Callable(self, "_on_filter_button_toggled"))
		portrait_filters.add_child(btnp)
		var btnl: PlaceFilterButton = PLACE_CATEGORY_FILTER_BUTTON.instantiate()
		btnl.button_group = group
		btnl.toggle_mode = true
		btnl.filter_type = i
		btnl.connect("filter_toggled", Callable(self, "_on_filter_button_toggled"))
		landscape_filters.add_child(btnl)
			
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
		spawn_pin(13, poi_places[i])
		
	var live_places = await async_load_category('live')
	live_places_ids = live_places.map(func(live_place): return live_place.id )
	for i in range(live_places.size()):
		spawn_pin(14, live_places[i])	
		
	_async_draw_archipelagos()
	#var circle = ARCHIPELAGO_CIRCLE.instantiate()
	
func _on_viewport_resized()->void:
	update_layout()
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

func _process(_delta):
	if map_is_on_top and margin_container.visible:
		animation_player.play('hide_filters')
	elif not map_is_on_top and not margin_container.visible:
		animation_player.play('show_filters')

func center_camera_on_parcel(parcel:Vector2i) -> void:
	var zoom_on_parcel = MAX_ZOOM
	var target_position = get_parcel_position(parcel)
	var tween = create_tween()
	tween.tween_property(camera, "position", target_position, 0.3).set_trans(Tween.TRANS_LINEAR).set_ease(Tween.EASE_OUT)
	tween.tween_property(camera, "zoom", zoom_on_parcel, 0.3).set_trans(Tween.TRANS_LINEAR).set_ease(Tween.EASE_OUT)
	
	await tween.finished
	update_label_settings(zoom_on_parcel)
	
func handle_click(event_position:Vector2)-> void:
	var coords = event_position / PARCEL_SIZE
	var parcel_coords = Vector2i(coords) - PARCEL_OFFSET
	clicked_parcel.emit(parcel_coords)
	show_cursor_at_parcel(parcel_coords)

func get_parcel_position(parcel: Vector2i) -> Vector2:
	var parcel_position = Vector2(parcel + PARCEL_OFFSET) * PARCEL_SIZE + PARCEL_SIZE / 2
	return parcel_position

func show_cursor_at_parcel(parcel: Vector2i):
	center_camera_on_parcel(parcel)
	var pos = get_parcel_position(parcel)
	cursor_marker.position = pos
	cursor_marker.visible = true
	
	coordinates_label.text = '%s, %s' % [parcel.x, -parcel.y]
	coordinates_label.show()
	update_label_settings(camera.zoom)

func update_label_settings(target_zoom) -> void:
	const FONT_SIZE = 18
	const OUTLINE_SIZE = 6
	coordinates_label.label_settings.font_size = int(FONT_SIZE / target_zoom.x)
	coordinates_label.label_settings.outline_size = int(OUTLINE_SIZE / target_zoom.x)

func spawn_pin(category:int, place):
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
	pin.pin_category = category
	pin.z_index = cursor_marker.z_index+1
	
	pin.position = pos
	map.add_child(pin)

func create_place_card(place)->void:
	var item = DISCOVER_CARROUSEL_ITEM.instantiate()
	item.item_pressed.connect(_item_pressed)
	cards_v_box_container.add_child(item)
	item.set_data(place)

func _item_pressed(place)->void:
	show_cursor_at_parcel(get_center_from_rect_coords(place.positions))

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
	if not pressed:
		filtered_places = []
		_clean_list_and_pins()
		portrait_map_searchbar.reset()
		landscape_map_searchbar.reset()
		_close_sidebar()
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
				continue
			spawn_pin(type, place)
			
		if places_to_show == 0:
			no_results.show()
			cards_scroll_container.hide()
		else:
			no_results.hide()
			cards_scroll_container.show()
		_open_sidebar()
	
		
		portrait_map_searchbar.filter_type = type
		portrait_map_searchbar.update_filtered_category()
		landscape_map_searchbar.filter_type = type
		landscape_map_searchbar.update_filtered_category()
		
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
	filtered_places = []

func _close_from_searchbar():
	for child in landscape_filters.get_children():
		if child is Button and child.toggle_mode and child.filter_type == active_filter:
			child.button_pressed = false
	for child in portrait_filters.get_children():
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
				continue
			spawn_pin(0, place)
	if places_to_show == 0:
		no_results.show()
		cards_scroll_container.hide()
	else:
		no_results.hide()
		cards_scroll_container.show()
	_open_sidebar()

func _clean_list_and_pins()->void:
	for child in cards_v_box_container.get_children():
		child.queue_free()
	for child in map.get_children():
		if child is MapPin:
			if child.pin_category != 13 and child.pin_category != 14 :
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
				center_coord = get_center_from_rect_coords_array(archipelago.parcels)
			else:
				var x = archipelago.parcels[0][0]
				var y = -archipelago.parcels[0][1]
				center_coord = Vector2i(x,-y)
			var radius = 50 + archipelago.usersTotalCount * 10
			var pos = get_parcel_position(center_coord)
			archipelagos_control.add_child(circle)
			circle.set_circle(pos, radius)
			
			
func get_center_from_rect_coords_array(coords: Array) -> Vector2i:
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


func _on_check_box_toggled(toggled_on: bool) -> void:
	archipelagos_control.visible = toggled_on
	update_layout()


func update_layout()->void:
	if Global.is_orientation_portrait():
		portrait.show()
		landscape.hide()
	else:
		portrait.hide()
		landscape.show()


func _on_map_gui_input(event):
	if event is InputEventScreenTouch:
		if event.pressed:
			active_touches[event.index] = event.position

			if active_touches.size() == 1:
				last_pan_position = event.position
				pan_started = false
				just_zoomed = false

			elif active_touches.size() == 2:
				# El segundo dedo acaba de tocar: inicializamos la distancia de zoom
				var positions = active_touches.values()
				last_pinch_distance = positions[0].distance_to(positions[1])

		else:
			if active_touches.has(event.index):
				var released_position = event.position
				var distance = released_position.distance_to(last_pan_position)
				if distance < PAN_THRESHOLD and not pan_started and not just_zoomed:
					handle_tap(released_position)

				active_touches.erase(event.index)

			# Si queda menos de 2 dedos, desactivamos el modo zoom
			if active_touches.size() < 2:
				last_pinch_distance = 0.0

	elif event is InputEventScreenDrag:
		if active_touches.has(event.index):
			active_touches[event.index] = event.position

			if active_touches.size() == 1:
				var distance = event.position.distance_to(last_pan_position)
				if distance >= PAN_THRESHOLD and not just_zoomed:
					pan_started = true
					handle_pan(event)

			elif active_touches.size() == 2:
				handle_zoom()

func handle_pan(event: InputEventScreenDrag):
	var delta = event.position - last_pan_position
	camera.position -= delta / camera.zoom
	last_pan_position = event.position

func handle_zoom():
	var touch_positions = active_touches.values()
	var p1 = touch_positions[0]
	var p2 = touch_positions[1]
	var current_distance = p1.distance_to(p2)

	if last_pinch_distance > 0.0 and current_distance > 0.0:
		var delta_ratio = (current_distance - last_pinch_distance) / last_pinch_distance

		# Filtramos valores extremos (esto cubre vibraciones)
		if abs(delta_ratio) > 0.01 and abs(delta_ratio) < 0.5:
			apply_zoom(delta_ratio)
			just_zoomed = true

	# Actualizamos la distancia suavemente
	last_pinch_distance = lerp(last_pinch_distance, current_distance, 0.5)

func apply_zoom(delta_ratio: float):
	var zoom_strength = 1.0  # delta_ratio ya es proporcional
	var zoom_factor = 1.0 + delta_ratio * zoom_strength

	var new_zoom = camera.zoom * Vector2(zoom_factor, zoom_factor)
	camera.zoom = new_zoom.clamp(Vector2(0.5, 0.5), Vector2(4, 4))

func handle_tap(pos: Vector2):
	print(pos)
	var coords = pos / PARCEL_SIZE
	var parcel_coords = Vector2i(coords) - PARCEL_OFFSET
	clicked_parcel.emit(parcel_coords)
	show_cursor_at_parcel(parcel_coords)
