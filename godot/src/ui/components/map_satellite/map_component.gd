extends Control

signal map_tapped(world_position: Vector2)

@onready var camera := $Camera2D

const MIN_ZOOM := Vector2(.5, .5)
const MAX_ZOOM := Vector2(2, 2)
const TAP_THRESHOLD := 10.0  # en píxeles
var dragging := false
var touch_start_pos := Vector2.ZERO
var touch_id := -1
var last_pinch_distance := 0.0
var active_touches := {}
var just_zoomed := false

func _ready():
	set_process_input(true)

func _input(event):
	# --- Tacto: inicio o fin de un toque ---
	if event is InputEventScreenTouch:
		if event.pressed:
			active_touches[event.index] = event.position
			if active_touches.size() == 1:
				touch_start_pos = event.position
				touch_id = event.index
				dragging = false
		else:
			active_touches.erase(event.index)
			if event.index == touch_id:
				var distance = touch_start_pos.distance_to(event.position)
				if distance < TAP_THRESHOLD:
					var world_pos = to_local_position(event.position)
					print("Emitiendo tap desde index:%d" % event.index)
					emit_signal("map_tapped", world_pos)
					camera.position = world_pos
				touch_id = -1
			if active_touches.size() < 2:
				last_pinch_distance = 0.0

	# --- Movimiento del dedo ---
	elif event is InputEventScreenDrag:
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
