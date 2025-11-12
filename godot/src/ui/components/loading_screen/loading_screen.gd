extends Control

var bg_colors: Array[Color] = [
	Color(0.5, 0.25, 0.0, 1.0),
	Color(0.0, 0.0, 0.5, 1.0),
	Color(0.5, 0.5, 0.0, 1.0),
	Color(0.5, 0.0, 0.25, 1.0),
	Color(0.25, 0.0, 0.5, 1.0),
	Color(0.34, 0.22, 0.15, 1.0),
	Color(0.0, 0.5, 0.5, 1.0),
]

var item_index = 0
var item_count = 0
var progress: float = 0.0
var last_progress_change := Time.get_ticks_msec()
var popup_warning_pos_y: int = 0

var last_hide_click := 0.0

var loaded_resources_offset := 0

# Benchmark locations for multi-location testing
var benchmark_locations = [
	{"name": "Goerli Plaza", "pos": Vector2i(72, -10), "realm": "https://sdk-team-cdn.decentraland.org/ipfs/goerli-plaza-main-latest"},
	{"name": "Genesis Plaza", "pos": Vector2i(0, 0), "realm": "https://realm-provider-ea.decentraland.org/main"},
	#{"name": "Goerli Plaza (Cleanup Test)", "pos": Vector2i(72, -10), "realm": "https://sdk-team-cdn.decentraland.org/ipfs/goerli-plaza-main-latest"}
]
var current_benchmark_index = 0
var benchmark_locations_tested = false

@onready var loading_progress = %ColorRect_LoadingProgress
@onready var loading_progress_label = %Label_LoadingProgress
@onready var label_loading_state = %Label_LoadingState

@onready
var carousel = $VBox_Loading/ColorRect_Background/Control_Discover/VBoxContainer/HBoxContainer_Content/CarouselViewport/SubViewport/Carousel
@onready var background: ColorRect = $VBox_Loading/ColorRect_Background

@onready var timer_auto_move_carousel = $Timer_AutoMoveCarousel
@onready var popup_warning = $PopupWarning

@onready var loading_screen_progress_logic = $LoadingScreenProgressLogic
@onready var timer_check_progress_timeout = $Timer_CheckProgressTimeout
@onready var debug_chronometer := Chronometer.new()


func _ready():
	last_progress_change = Time.get_ticks_msec()
	popup_warning.hide()
	popup_warning_pos_y = popup_warning.position.y
	item_count = carousel.item_count()
	set_item(randi_range(0, item_count - 1))


# Forward
func enable_loading_screen():
	if !debug_chronometer:
		debug_chronometer = Chronometer.new()
	debug_chronometer.restart("Starting to load scene")
	Global.loading_started.emit()
	Global.release_mouse()
	loading_screen_progress_logic.enable_loading_screen()


func async_hide_loading_screen_effect():
	debug_chronometer.lap("Finished loading scene")
	Global.loading_finished.emit()
	timer_check_progress_timeout.stop()
	var tween = get_tree().create_tween()
	background.use_parent_material = true  # disable material
	modulate = Color.WHITE
	tween.tween_property(self, "modulate", Color.TRANSPARENT, 1.0)
	await tween.finished
	hide()
	modulate = Color.WHITE
	background.use_parent_material = false  # enable material
	self.position.y = 0

	if Global.cli.measure_perf:
		var counter = (
			load("res://addons/dcl_dev_tools/dev_tools/resource_counter/resource_counter.gd").new()
		)
		add_child(counter)
		await get_tree().create_timer(5).timeout
		counter.log_active_counts()

	if Global.cli.benchmark_report:
		# Wait 5 seconds for scene to fully load
		await get_tree().create_timer(5).timeout

		# Collect resource data from GDScript
		var counter = (
			load("res://addons/dcl_dev_tools/dev_tools/resource_counter/resource_counter.gd").new()
		)
		add_child(counter)
		counter.count(get_tree().get_root().get_node("scene_runner"))

		# Prepare resource data dictionary
		var resource_data = {
			"total_meshes": counter.meshes.size(),
			"total_materials": counter.materials.size(),
			"mesh_rid_count": 0,
			"material_rid_count": 0,
			"mesh_hash_count": 0,
			"potential_dedup_count": 0,
			"mesh_savings_percent": 0.0
		}

		# Calculate RID counts and deduplication metrics (simplified)
		var mesh_rid_map = {}
		for mesh in counter.meshes:
			if mesh:
				var mesh_rid = mesh.get_rid()
				mesh_rid_map[mesh_rid] = true
		resource_data["mesh_rid_count"] = mesh_rid_map.size()

		var material_rid_map = {}
		for material in counter.materials:
			if material:
				var material_rid = material.get_rid()
				material_rid_map[material_rid] = true
		resource_data["material_rid_count"] = material_rid_map.size()

		# Get benchmark reporter from Global
		var benchmark_report = Global.benchmark_report
		if not benchmark_report:
			# BenchmarkReport should be created as a singleton in Global
			print("Warning: BenchmarkReport not found in Global")
			counter.queue_free()
			return

		# Get current location info
		var current_pos = Global.get_explorer().parcel_position
		var location_name = benchmark_locations[current_benchmark_index].name

		# Collect metrics with resource data (numbered prefix for sorting)
		var test_name = "4_Explorer_" + str(current_pos) + "_" + location_name.replace(" ", "_")
		var location = str(current_pos)
		var realm = Global.realm.get_realm_string()

		benchmark_report.collect_and_store_metrics(test_name, location, realm, resource_data)
		benchmark_report.generate_individual_report()

		print("✓ Explorer benchmark collected at " + str(current_pos) + " (" + location_name + ")")

		# Clean up
		counter.queue_free()

		# Check if we need to test more locations
		_check_next_benchmark_location()


func _on_texture_rect_right_arrow_gui_input(event):
	if event is InputEventScreenTouch:
		if event.pressed:
			next_page()


func _on_texture_rect_left_arrow_gui_input(event):
	if event is InputEventScreenTouch:
		if event.pressed:
			prev_page()


func prev_page():
	var next_index = item_index - 1
	if next_index < 0:
		next_index = item_count - 1

	set_item(next_index, false)


func next_page():
	var next_index = item_index + 1
	if next_index >= item_count:
		next_index = 0

	set_item(next_index, true)


func set_item(index: int, right_direction: bool = true):
	timer_auto_move_carousel.stop()
	timer_auto_move_carousel.start()

	var current_color = Color(bg_colors[item_index])
	item_index = index
	carousel.set_item(index, right_direction)

	var new_color = Color(bg_colors[index])
	set_bg_shader_color(current_color, new_color)


func set_bg_shader_color(from: Color, to: Color):
	var tween = get_tree().create_tween()
	tween.tween_method(set_shader_background_color, from, to, 0.25)


func set_shader_background_color(color: Color):
	var shader_material: ShaderMaterial = background.get_material()
	shader_material.set_shader_parameter("lineColor", color)


func set_progress(new_progress: float):
	new_progress = clampf(new_progress, 0.0, 100.0)
	if progress != new_progress:
		last_progress_change = Time.get_ticks_msec()
	progress = new_progress

	loading_progress_label.text = "LOADING %d%%" % floor(progress)
	var tween = get_tree().create_tween()
	var new_width = loading_progress.get_parent().size.x * (progress / 100.0)
	tween.tween_property(loading_progress, "position:x", new_width, 0.1)


func _on_timer_auto_move_carousel_timeout():
	next_page()


func _on_timer_check_progress_timeout_timeout():
	if Global.scene_runner.is_paused():
		last_progress_change = Time.get_ticks_msec()
		return

	var loading_resources = (
		Global.content_provider.count_loading_resources() - loaded_resources_offset
	)
	var loaded_resources = (
		Global.content_provider.count_loaded_resources() - loaded_resources_offset
	)
	var download_speed_mbs: float = Global.content_provider.get_download_speed_mbs()
	label_loading_state.text = (
		"(%d/%d resources at %.2fmb/s)" % [loaded_resources, loading_resources, download_speed_mbs]
	)

	var inactive_seconds: int = int(floor((Time.get_ticks_msec() - last_progress_change) / 1000.0))
	if inactive_seconds > 20:
		var tween = get_tree().create_tween()
		popup_warning.position.y = -popup_warning.size.y
		tween.tween_property(popup_warning, "position:y", popup_warning_pos_y, 1.0).set_trans(
			Tween.TRANS_ELASTIC
		)
		popup_warning.show()
		timer_check_progress_timeout.stop()


func async_hide_popup_warning():
	var tween = get_tree().create_tween()
	popup_warning.position.y = popup_warning_pos_y
	tween.tween_property(popup_warning, "position:y", -popup_warning.size.y, 1.0).set_trans(
		Tween.TRANS_ELASTIC
	)
	await tween.finished
	popup_warning.hide()


func _on_button_continue_pressed():
	loading_screen_progress_logic.hide_loading_screen()
	async_hide_popup_warning()


func _on_button_reload_pressed():
	Global.realm.async_set_realm(Global.realm.get_realm_string())
	async_hide_popup_warning()


func _on_loading_screen_progress_logic_loading_show_requested():
	last_progress_change = Time.get_ticks_msec()
	popup_warning.hide()
	timer_check_progress_timeout.start()
	loaded_resources_offset = Global.content_provider.count_loaded_resources()


# For dev purposes
func _on_color_rect_header_gui_input(event):
	if event is InputEventScreenTouch:
		if event.pressed:
			var elapsed_time = Time.get_ticks_msec() - last_hide_click
			if elapsed_time <= 500:
				loading_screen_progress_logic.hide_loading_screen()
			last_hide_click = Time.get_ticks_msec()


# Benchmark helper functions
func _update_benchmark_location_index(current_pos: Vector2i):
	# Find which location we're currently at
	for i in range(benchmark_locations.size()):
		if benchmark_locations[i].pos == current_pos:
			current_benchmark_index = i
			return


func _check_next_benchmark_location():
	if not Global.cli.benchmark_report:
		return

	current_benchmark_index += 1

	if current_benchmark_index < benchmark_locations.size():
		# More locations to test
		var next_loc = benchmark_locations[current_benchmark_index]
		print("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━")
		print("✓ Moving to next benchmark location: %s at %s (realm: %s)" % [next_loc.name, next_loc.pos, next_loc.realm])
		print("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━")
		await get_tree().create_timer(2.0).timeout
		Global.teleport_to(next_loc.pos, next_loc.realm)
	else:
		# All locations tested - generate summary and quit
		print("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━")
		print("✓ All benchmark locations completed!")
		print("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━")
		_finalize_benchmark()


func _finalize_benchmark():
	var benchmark_report = Global.benchmark_report
	if not benchmark_report:
		print("⚠ BenchmarkReport not found, cannot generate summary")
		await get_tree().create_timer(2.0).timeout
		get_tree().quit()
		return

	print("Generating comprehensive summary report...")
	benchmark_report.generate_summary_report()

	print("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━")
	print("✅ BENCHMARK COMPLETE!")
	print("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━")
	print("Reports saved to user data directory:")
	print("  - Individual reports for each stage")
	print("  - Comprehensive summary report")
	print("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━")

	await get_tree().create_timer(3.0).timeout
	get_tree().quit()
