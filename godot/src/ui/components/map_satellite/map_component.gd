extends Control

signal clicked_parcel(parcel: Vector2i)

@onready var camera := $Camera2D
@onready var archipelagos_control: Control = %ArchipelagosControl

const IMAGE_FOLDER = "res://src/ui/components/map_satellite/assets/4/"
const TILE_SIZE = Vector2(512, 512)
const GRID_SIZE = Vector2(16, 16)
const PARCELS_PER_TILE = Vector2(20, 20)
const PARCEL_SIZE = TILE_SIZE / PARCELS_PER_TILE
const MIN_ZOOM := Vector2(.5, .5)
const MAX_ZOOM := Vector2(2, 2)
const TAP_THRESHOLD := 10.0  # en píxeles
const MAP_SIZE = PARCEL_SIZE * Vector2(340,340)
const PARCEL_OFFSET = Vector2i(170,170)
const MAP_CENTER = MAP_SIZE / 2
const TILE_DISPLACEMENT = Vector2(18,18) * PARCEL_SIZE
const MAP_MARKER = preload("res://src/ui/components/map_satellite/map_marker.tscn")
const MAP_PIN := preload("res://src/ui/components/map_satellite/map_pin.tscn")
const ARCHIPELAGO_CIRCLE = preload("res://src/ui/components/map_satellite/archipelago_circle.tscn")
var dragging := false
var touch_start_pos := Vector2.ZERO
var touch_id := -1
var last_pinch_distance := 0.0
var active_touches := {}
var just_zoomed := false
var poi_places_ids = []


func _ready():
	set_process_input(true)
	self.size = MAP_SIZE
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
				add_child(tex_rect)
			else:
				push_error("Error loading map image: " + image_path)
	
	center_camera_on_parcel(Vector2i(0,1))
	
func _input(event):
	# --- Movimiento del dedo ---
	if event is InputEventScreenDrag:
		active_touches[event.index] = event.position

		if active_touches.size() == 1 and event.index == touch_id:
			dragging = true
			camera.position -= event.relative / camera.zoom

		elif active_touches.size() == 2:
			handle_zoom()

	# --- Zoom con rueda del mouse ---
	elif event is InputEventMouseButton:
		if event.button_index == MOUSE_BUTTON_WHEEL_UP:
			camera.zoom = clamp(camera.zoom * 1.1, MIN_ZOOM, MAX_ZOOM)
		elif event.button_index == MOUSE_BUTTON_WHEEL_DOWN:
			camera.zoom = clamp(camera.zoom * 0.9, MIN_ZOOM, MAX_ZOOM)

func to_local_position(screen_pos: Vector2) -> Vector2:
	var canvas_xform := get_canvas_transform()
	return canvas_xform.affine_inverse() * screen_pos

func handle_zoom():
	var touch_positions = active_touches.values()
	if touch_positions.size() < 2:
		return

	var p1 = touch_positions[0]
	var p2 = touch_positions[1]
	var current_distance = p1.distance_to(p2)

	if last_pinch_distance > 0.0:
		var delta_ratio = (current_distance - last_pinch_distance) / last_pinch_distance
		if abs(current_distance - last_pinch_distance) > 2.0 and abs(delta_ratio) < 0.5:
			apply_zoom(delta_ratio)
			just_zoomed = true

	last_pinch_distance = current_distance

func apply_zoom(delta_ratio: float):
	var zoom_strength = 1.0
	var zoom_factor = 1.0 + delta_ratio * zoom_strength
	var new_zoom = camera.zoom * Vector2(zoom_factor, zoom_factor)
	camera.zoom = new_zoom.clamp(MIN_ZOOM, MAX_ZOOM)

func handle_tap(pos: Vector2):
	var coords = pos / PARCEL_SIZE
	var parcel_coords = Vector2i(coords) - PARCEL_OFFSET
	clicked_parcel.emit(parcel_coords)
	show_marker_at_parcel(parcel_coords)

func _on_gui_input(event: InputEvent) -> void:
	# --- Tacto: inicio o fin de un toque ---
	if event is InputEventScreenTouch:
		var mouse_position = get_viewport().get_mouse_position()
		if event.pressed:
			active_touches[event.index] = event.position
			if active_touches.size() == 1:
				touch_start_pos = mouse_position
				prints("Mouse down:", touch_start_pos)
				touch_id = event.index
				dragging = false
		else:
			active_touches.erase(event.index)
			if event.index == touch_id:
				var distance = touch_start_pos.distance_to(mouse_position)
				prints("Distance", distance, touch_start_pos, mouse_position)
				if distance < TAP_THRESHOLD:
					var world_pos = event.position
					print("Emitiendo tap desde index:%d" % event.index)
					handle_tap(world_pos)
					#camera.position = world_pos
				touch_id = -1
			if active_touches.size() < 2:
				last_pinch_distance = 0.0

func async_draw_archipelagos() -> void:
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



func get_poi_ids(poi_places):
	poi_places_ids = poi_places.map(func(poi_place): return poi_place.id)
	
func create_pins(category:int, places:Array, group_name:String)->void:
	
	for i in range(places.size()):
		print(poi_places_ids)
		if poi_places_ids.has(places[i].id):
			print('is poi')
			spawn_pin(category, places[i], 'hidden')
		else:
			spawn_pin(category, places[i], group_name)

func _get_center_from_rect_coords(coords: Array) -> Vector2i:
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

func get_parcel_position(parcel: Vector2i) -> Vector2:
	var parcel_position = Vector2(parcel + PARCEL_OFFSET) * PARCEL_SIZE + PARCEL_SIZE / 2
	return parcel_position

func spawn_pin(category:int, place, group:String):
	var pin = MAP_PIN.instantiate()
	var center_coord:Vector2i
	if category != 14:
		pin.scene_title = place.title
		if place.positions.size() != 1:
			center_coord = _get_center_from_rect_coords(place.positions)
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
	pin.z_index = category + center_coord.y + MAP_SIZE.y
	
	pin.position = pos
	add_child(pin)
	pin.add_to_group(group)
	if group == 'hidden':
		pin.visible = false

func center_camera_on_parcel(parcel:Vector2i) -> void:
	var target_position = get_parcel_position(parcel)
	var tween = create_tween()
	tween.tween_property(camera, "position", target_position, 0.3).set_trans(Tween.TRANS_LINEAR).set_ease(Tween.EASE_OUT)
	await tween.finished

func show_marker_at_parcel(parcel: Vector2i):
	prints("Show market at parcel", parcel)
	for child in get_children():
		if child is Marker:
			child.queue_free()
			
	center_camera_on_parcel(parcel)
	var marker = MAP_MARKER.instantiate()
	var pos = get_parcel_position(parcel) - marker.size / 2
	marker.position = pos	
	marker.marker_x = parcel.x
	marker.marker_y = -parcel.y
	add_child(marker)
	marker.visible = true
	marker.update()

func _on_pin_clicked(coord:Vector2i):
	clicked_parcel.emit(coord)
	show_marker_at_parcel(coord)

func card_pressed(place)->void:
	show_marker_at_parcel(_get_center_from_rect_coords(place.positions))

func clamp_camera_position() -> void:
	const MAP_TOP_MARGIN = 50
	var min_pos:Vector2 = size / 2 / camera.zoom
	var max_pos:Vector2 = MAP_SIZE - min_pos

	if MAP_SIZE.x * camera.zoom.x < size.x:
		camera.position.x = ( size.x  + MAP_SIZE.x ) / 2
	else:
		camera.position.x = clamp(camera.position.x, min_pos.x, max_pos.x)

	if MAP_SIZE.y * camera.zoom.y < size.y:
		camera.position.y = ( size.y  + MAP_SIZE.y ) / 2
	else:
		camera.position.y = clamp(camera.position.y, min_pos.y, max_pos.y)

func clear_pins()->void:
	for child in get_children():
		if child.is_in_group('pins') or child.is_in_group('hidden'):
			child.queue_free() 

func show_poi_toggled(toggled_on: bool) -> void:
	for child in get_children():
		if child.is_in_group('poi_pins'):
			child.visible = toggled_on
		if child.is_in_group('hidden'):
			child.visible = !toggled_on

func show_live_toggled(toggled_on: bool) -> void:
	for child in get_children():
		if child.is_in_group('live_pins'):
			child.visible = toggled_on
