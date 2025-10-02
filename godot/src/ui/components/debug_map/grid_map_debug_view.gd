class_name GridMapDebugView
extends Control

# Track parcel states
enum ParcelType { NOT_LOADED, MAIN_SCENE, EMPTY_PARCEL, KEEP_ALIVE_SCENE }

# Colors for different parcel states
const COLOR_LOADED_SCENE = Color.WHITE
const COLOR_EMPTY_PARCEL = Color.GREEN
const COLOR_KEEP_ALIVE_SCENE = Color.RED  # Adjacent loaded scenes
const COLOR_NOT_LOADED = Color(0.1, 0.1, 0.1)  # Dark gray/black
const COLOR_PLAYER = Color.ORANGE
const COLOR_GRID = Color(0.3, 0.3, 0.3)
const COLOR_BACKGROUND = Color(0.05, 0.05, 0.05)
const COLOR_ORIGIN = Color(0.5, 0.5, 0.8, 0.5)  # Blue for (0,0)

@export var view_radius: int = 15  # How many parcels to show in each direction
@export var cell_size: float = 10.0  # Size of each cell in pixels
@export var player_position: Vector2 = Vector2.ZERO  # Exact player position (not grid-snapped)
@export var player_rotation: float = 0.0  # Player/camera rotation in radians
@export var camera_center: Vector2 = Vector2.ZERO  # Center of the view
@export var zoom_level: float = 1.0
@export var follow_player: bool = true  # Whether camera follows player

var loaded_parcels: Dictionary = {}  # Dict[Vector2i, ParcelType]
var viewport_size: Vector2


func _ready():
	custom_minimum_size = Vector2(300, 300)
	size = Vector2(300, 300)
	viewport_size = size

	# Connect to SceneFetcher for updates
	if Global.scene_fetcher:
		Global.scene_fetcher.parcels_processed.connect(_on_parcels_processed)


func _draw():
	# Draw background
	draw_rect(Rect2(Vector2.ZERO, viewport_size), COLOR_BACKGROUND)

	# Update camera center if following player
	if follow_player:
		camera_center = player_position

	# Calculate visible range (using integer bounds for grid)
	var min_x = int(camera_center.x) - view_radius
	var max_x = int(camera_center.x) + view_radius
	var min_y = int(camera_center.y) - view_radius
	var max_y = int(camera_center.y) + view_radius

	var center_offset = viewport_size / 2

	# Draw grid cells
	for x in range(min_x, max_x + 1):
		for y in range(min_y, max_y + 1):
			var world_pos = Vector2i(x, y)

			# Calculate screen position relative to camera center (flip Y for display)
			var relative_pos = Vector2(
				(x - camera_center.x) * cell_size * zoom_level,
				-(y - camera_center.y) * cell_size * zoom_level  # Flip Y axis
			)
			var screen_pos = center_offset + relative_pos

			# Skip if outside viewport
			if screen_pos.x < -cell_size * zoom_level or screen_pos.x > viewport_size.x:
				continue
			if screen_pos.y < -cell_size * zoom_level or screen_pos.y > viewport_size.y:
				continue

			# Determine cell color based on parcel state
			var cell_color = COLOR_NOT_LOADED
			if loaded_parcels.has(world_pos):
				match loaded_parcels[world_pos]:
					ParcelType.MAIN_SCENE:
						cell_color = COLOR_LOADED_SCENE
					ParcelType.EMPTY_PARCEL:
						cell_color = COLOR_EMPTY_PARCEL
					ParcelType.KEEP_ALIVE_SCENE:
						cell_color = COLOR_KEEP_ALIVE_SCENE
					_:
						cell_color = COLOR_NOT_LOADED

			# Draw cell
			var cell_rect = Rect2(
				screen_pos, Vector2(cell_size * zoom_level, cell_size * zoom_level)
			)
			draw_rect(cell_rect, cell_color)

			# Draw grid lines
			draw_rect(cell_rect, COLOR_GRID, false, 1.0)

			# Highlight origin (0,0)
			if world_pos == Vector2i.ZERO:
				draw_rect(cell_rect, COLOR_ORIGIN, false, 2.0)

	# Draw player as a directional triangle (not grid-snapped)
	var player_relative_pos = Vector2(
		(player_position.x - camera_center.x) * cell_size * zoom_level,
		-(player_position.y - camera_center.y) * cell_size * zoom_level  # Restore Y flip for consistency with grid
	)
	var player_screen_pos = center_offset + player_relative_pos

	# Draw player only if visible
	if (
		player_screen_pos.x >= -10
		and player_screen_pos.x <= viewport_size.x + 10
		and player_screen_pos.y >= -10
		and player_screen_pos.y <= viewport_size.y + 10
	):
		# Draw a triangle pointing in the camera direction
		var triangle_size = 4.0 * zoom_level

		# Create triangle points (pointing up by default)
		var points = PackedVector2Array()
		points.append(Vector2(0, -triangle_size))  # Front point
		points.append(Vector2(-triangle_size * 0.6, triangle_size * 0.6))  # Back left
		points.append(Vector2(triangle_size * 0.6, triangle_size * 0.6))  # Back right

		# Rotate triangle based on player rotation
		var rotated_points = PackedVector2Array()
		for point in points:
			var rotated = point.rotated(player_rotation)
			rotated_points.append(player_screen_pos + rotated)

		# Draw filled triangle
		draw_colored_polygon(rotated_points, COLOR_PLAYER)

		# Draw triangle outline for better visibility
		draw_polyline(rotated_points, Color(0.5, 0, 0), 1.0, true)
		rotated_points.append(rotated_points[0])  # Close the triangle
		draw_polyline(rotated_points, Color(0.5, 0, 0), 1.0)

	# Draw coordinates text
	var coord_text = "Player: (%.2f, %.2f)" % [player_position.x, player_position.y]
	if not follow_player:
		coord_text += " | View: (%.1f, %.1f)" % [camera_center.x, camera_center.y]
	draw_string(
		get_theme_default_font(),
		Vector2(5, viewport_size.y - 10),
		coord_text,
		HORIZONTAL_ALIGNMENT_LEFT,
		-1,
		12,
		Color.WHITE
	)

	# Draw legend
	_draw_legend()


func _draw_legend():
	var legend_y = 5.0
	var legend_x = viewport_size.x - 80
	var legend_size = 10.0

	# Scene
	draw_rect(
		Rect2(Vector2(legend_x, legend_y), Vector2(legend_size, legend_size)), COLOR_LOADED_SCENE
	)
	draw_string(
		get_theme_default_font(),
		Vector2(legend_x + legend_size + 5, legend_y + 10),
		"Scene",
		HORIZONTAL_ALIGNMENT_LEFT,
		-1,
		10,
		Color.WHITE
	)

	# Empty
	legend_y += 15
	draw_rect(
		Rect2(Vector2(legend_x, legend_y), Vector2(legend_size, legend_size)), COLOR_EMPTY_PARCEL
	)
	draw_string(
		get_theme_default_font(),
		Vector2(legend_x + legend_size + 5, legend_y + 10),
		"Empty",
		HORIZONTAL_ALIGNMENT_LEFT,
		-1,
		10,
		Color.WHITE
	)

	# Keep Alive
	legend_y += 15
	draw_rect(
		Rect2(Vector2(legend_x, legend_y), Vector2(legend_size, legend_size)),
		COLOR_KEEP_ALIVE_SCENE
	)
	draw_string(
		get_theme_default_font(),
		Vector2(legend_x + legend_size + 5, legend_y + 10),
		"Keep Alive",
		HORIZONTAL_ALIGNMENT_LEFT,
		-1,
		10,
		Color.WHITE
	)

	# Player
	legend_y += 15
	draw_circle(Vector2(legend_x + legend_size / 2, legend_y + legend_size / 2), 3.0, COLOR_PLAYER)
	draw_string(
		get_theme_default_font(),
		Vector2(legend_x + legend_size + 5, legend_y + 10),
		"Player",
		HORIZONTAL_ALIGNMENT_LEFT,
		-1,
		10,
		Color.WHITE
	)


func update_player_position(pos: Vector2):
	player_position = pos
	queue_redraw()


func update_player_rotation(rot: float):
	player_rotation = rot
	queue_redraw()


func update_player_transform(pos: Vector2, rot: float):
	player_position = pos
	player_rotation = rot
	queue_redraw()


func _on_parcels_processed(parcel_filled: Array, _empty_parcels: Array):
	# Clear and update loaded parcels
	loaded_parcels.clear()

	# Add scene parcels (actual loaded scenes)
	for parcel in parcel_filled:
		if parcel is Vector2i:
			loaded_parcels[parcel] = ParcelType.MAIN_SCENE

	# Add empty parcels (actual instantiated empty scenes)
	if Global.scene_fetcher:
		var scene_fetcher = Global.scene_fetcher
		for parcel_string in scene_fetcher.loaded_empty_scenes.keys():
			var coord = parcel_string.split(",")
			if coord.size() == 2:
				var x = int(coord[0])
				var z = int(coord[1])
				loaded_parcels[Vector2i(x, z)] = ParcelType.EMPTY_PARCEL

		# Add keep-alive scenes (adjacent loaded scenes that aren't main scenes)
		# Get the desired scenes from the coordinator
		var coordinator = scene_fetcher.scene_entity_coordinator
		var desired_scenes = coordinator.get_desired_scenes()
		var keep_alive_scenes = desired_scenes.get("keep_alive_scenes", [])

		for scene_id in keep_alive_scenes:
			var scene = scene_fetcher.loaded_scenes.get(scene_id)
			if scene != null:
				for parcel in scene.parcels:
					# Only mark as keep-alive if not already marked as main scene
					if not loaded_parcels.has(parcel):
						loaded_parcels[parcel] = ParcelType.KEEP_ALIVE_SCENE

	queue_redraw()


func set_zoom(new_zoom: float):
	zoom_level = clamp(new_zoom, 0.5, 2.0)
	queue_redraw()


func clear_parcels():
	loaded_parcels.clear()
	queue_redraw()


func set_follow_mode(enabled: bool):
	follow_player = enabled
	if enabled:
		camera_center = player_position
	queue_redraw()


func pan_camera(delta: Vector2):
	if not follow_player:
		camera_center += delta
		queue_redraw()


func center_on_player():
	camera_center = player_position
	queue_redraw()


func center_on_origin():
	camera_center = Vector2.ZERO
	follow_player = false
	queue_redraw()
