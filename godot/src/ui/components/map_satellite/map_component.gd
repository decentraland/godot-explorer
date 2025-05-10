extends Control

signal map_tapped(world_position: Vector2)

@onready var camera := $Camera2D

const MIN_ZOOM := Vector2(1, 1)
const MAX_ZOOM := Vector2(2, 2)
const TAP_THRESHOLD := 10.0  # en píxeles
var dragging := false
var touch_start_pos := Vector2.ZERO
var touch_id := -1
var last_pinch_distance := 0.0
var active_touches := {}
var just_zoomed:= false

func _ready():
	set_process_input(true)

func _input(event):
	# --- Tacto: inicio del toque ---
	if event is InputEventScreenTouch:
		if event.pressed:
			touch_start_pos = event.position
			touch_id = event.index
			dragging = false
		else:
			if touch_id == event.index:
				var distance = touch_start_pos.distance_to(event.position)
				if distance < TAP_THRESHOLD:
					var world_pos = to_local_position(event.position)
					camera.position = world_pos
					emit_signal("map_tapped", world_pos)
				touch_id = -1

	# --- Movimiento del dedo: detectar drag ---
	elif event is InputEventScreenDrag and event.index == touch_id:
		dragging = true
		camera.position -= event.relative / camera.zoom


	# --- Zoom con rueda del mouse ---
	elif event is InputEventMouseButton:
		if event.button_index == MOUSE_BUTTON_WHEEL_UP:
			camera.zoom = clamp(camera.zoom * 1.1, MIN_ZOOM, MAX_ZOOM)
		elif event.button_index == MOUSE_BUTTON_WHEEL_DOWN:
			camera.zoom = clamp(camera.zoom * 0.9, MIN_ZOOM, MAX_ZOOM)

	# --- Zoom con gesto de pinza (dos dedos) ---
	elif event is InputEventScreenDrag:
		if active_touches.has(event.index):
			if active_touches.size() == 2:
				handle_zoom()
	

func to_local_position(screen_pos: Vector2) -> Vector2:
	var canvas_xform := get_canvas_transform()
	return canvas_xform.affine_inverse() * screen_pos

func handle_zoom():
	var touch_positions = active_touches.values()
	var p1 = touch_positions[0]
	var p2 = touch_positions[1]
	var current_distance = p1.distance_to(p2)

	if last_pinch_distance > 0.0 and current_distance > 0.0:
		var delta_ratio = (current_distance - last_pinch_distance) / last_pinch_distance

		if abs(delta_ratio) > 0.01 and abs(delta_ratio) < 0.5:
			apply_zoom(delta_ratio)
			just_zoomed = true

	last_pinch_distance = lerp(last_pinch_distance, current_distance, 0.5)

func apply_zoom(delta_ratio: float):
	var zoom_strength = 1.0
	var zoom_factor = 1.0 + delta_ratio * zoom_strength
	var new_zoom = camera.zoom * Vector2(zoom_factor, zoom_factor)
	
	camera.zoom = new_zoom.clamp(MIN_ZOOM, MAX_ZOOM)
