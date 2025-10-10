class_name DebugMapContainer
extends SubViewportContainer

@export var enabled: bool = true:
	set(value):
		enabled = value
		visible = enabled

var grid_map_view: GridMapDebugView
var player_position_timer: Timer


func _ready():
	# Set up the container
	custom_minimum_size = Vector2(300, 300)
	size = Vector2(300, 300)
	stretch = true
	anchor_left = 1.0
	anchor_top = 0.0
	anchor_right = 1.0
	anchor_bottom = 0.0
	offset_left = -310
	offset_top = 10
	offset_right = -10
	offset_bottom = 310

	# Create SubViewport
	var viewport = SubViewport.new()
	viewport.size = Vector2(300, 300)
	viewport.render_target_update_mode = SubViewport.UPDATE_WHEN_VISIBLE
	add_child(viewport)

	# Create the grid map view
	grid_map_view = GridMapDebugView.new()
	viewport.add_child(grid_map_view)

	# Create timer for position updates
	player_position_timer = Timer.new()
	player_position_timer.wait_time = 0.1  # Update 10 times per second
	player_position_timer.timeout.connect(_update_player_position)
	add_child(player_position_timer)
	player_position_timer.start()

	# Add some UI overlay for controls
	_add_controls()


func _add_controls():
	# Add a panel for controls
	var control_panel = PanelContainer.new()
	control_panel.custom_minimum_size = Vector2(300, 30)
	control_panel.anchor_bottom = 1.0
	control_panel.anchor_top = 1.0
	control_panel.offset_top = -30
	control_panel.modulate = Color(1, 1, 1, 0.8)
	add_child(control_panel)

	var hbox = HBoxContainer.new()
	control_panel.add_child(hbox)

	# Toggle button
	var toggle_btn = Button.new()
	toggle_btn.text = "Hide"
	toggle_btn.custom_minimum_size = Vector2(40, 0)
	toggle_btn.pressed.connect(_on_toggle_pressed.bind(toggle_btn))
	hbox.add_child(toggle_btn)

	# Follow mode toggle
	var follow_btn = CheckBox.new()
	follow_btn.text = "Follow"
	follow_btn.button_pressed = true
	follow_btn.toggled.connect(_on_follow_toggled)
	hbox.add_child(follow_btn)

	# Zoom controls
	var zoom_out_btn = Button.new()
	zoom_out_btn.text = "-"
	zoom_out_btn.custom_minimum_size = Vector2(25, 0)
	zoom_out_btn.pressed.connect(_on_zoom_out)
	hbox.add_child(zoom_out_btn)

	var zoom_in_btn = Button.new()
	zoom_in_btn.text = "+"
	zoom_in_btn.custom_minimum_size = Vector2(25, 0)
	zoom_in_btn.pressed.connect(_on_zoom_in)
	hbox.add_child(zoom_in_btn)

	# Origin button
	var origin_btn = Button.new()
	origin_btn.text = "0,0"
	origin_btn.custom_minimum_size = Vector2(35, 0)
	origin_btn.tooltip_text = "Center on origin (0,0)"
	origin_btn.pressed.connect(_on_origin_pressed)
	hbox.add_child(origin_btn)

	# Clear button
	var clear_btn = Button.new()
	clear_btn.text = "Clear"
	clear_btn.custom_minimum_size = Vector2(45, 0)
	clear_btn.pressed.connect(_on_clear_pressed)
	hbox.add_child(clear_btn)


func _on_toggle_pressed(button: Button):
	if visible:  # SubViewport
		visible = false
		button.text = "Show"
	else:
		visible = true
		button.text = "Hide"


func _on_zoom_in():
	if grid_map_view:
		grid_map_view.set_zoom(grid_map_view.zoom_level + 0.25)


func _on_zoom_out():
	if grid_map_view:
		grid_map_view.set_zoom(grid_map_view.zoom_level - 0.25)


func _on_clear_pressed():
	if grid_map_view:
		grid_map_view.clear_parcels()


func _on_follow_toggled(pressed: bool):
	if grid_map_view:
		grid_map_view.set_follow_mode(pressed)


func _on_origin_pressed():
	if grid_map_view:
		grid_map_view.center_on_origin()


func _update_player_position():
	if not grid_map_view:
		return

	# Get actual player position from explorer (use Explorer's exact calculation)
	var explorer = Global.get_explorer()
	if explorer and explorer.player:
		# Use Explorer's exact parcel_position_real calculation with offset correction
		var parcel_pos = explorer.parcel_position_real
		# Apply offset correction for Y coordinate
		parcel_pos.y -= 1.0

		# Get camera rotation (Y rotation is the horizontal rotation)
		var camera_rot = 0.0
		if explorer.player.has_node("CameraRoot/PlayerCamera"):
			var camera = explorer.player.get_node("CameraRoot/PlayerCamera")
			# Get the global transform's Y rotation
			var basis = camera.get_global_transform().basis
			# Extract Y rotation (heading) from the basis
			# In Godot, when looking along -Z, atan2 gives us the heading
			var forward = -basis.z
			# Flip the X component to correct the left-right inversion
			camera_rot = atan2(-forward.x, forward.z)
		elif explorer.player.has_node("CameraContainer/PlayerCamera"):
			var camera = explorer.player.get_node("CameraContainer/PlayerCamera")
			var basis = camera.get_global_transform().basis
			var forward = -basis.z
			# Flip the X component to correct the left-right inversion
			camera_rot = atan2(-forward.x, forward.z)
		else:
			# Try to get rotation from player node itself
			camera_rot = -explorer.player.rotation.y  # Negate for correct direction

		grid_map_view.update_player_transform(parcel_pos, camera_rot)
	elif Global.scene_fetcher:
		# Fallback to scene fetcher position if player not available
		var player_pos = Global.scene_fetcher.current_position
		grid_map_view.update_player_position(Vector2(player_pos.x, player_pos.y))


func set_enabled(value: bool):
	enabled = value
	visible = enabled
	if player_position_timer:
		if enabled:
			player_position_timer.start()
		else:
			player_position_timer.stop()
