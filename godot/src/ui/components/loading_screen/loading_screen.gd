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
var last_activity_time := Time.get_ticks_msec()
var popup_warning_pos_y: int = 0

var last_hide_click := 0.0

var loaded_resources_offset := 0

@onready var loading_progress = %ColorRect_LoadingProgress
@onready var loading_progress_label = %Label_LoadingProgress
@onready var label_loading_state = %Label_LoadingState

@onready
var carousel = $VBox_Loading/ColorRect_Background/Control_Discover/VBoxContainer/HBoxContainer_Content/CarouselViewport/SubViewport/Carousel
@onready var background: ColorRect = $VBox_Loading/ColorRect_Background

@onready var timer_auto_move_carousel = $Timer_AutoMoveCarousel

@onready var loading_screen_progress_logic = $LoadingScreenProgressLogic
@onready var timer_check_progress_timeout = $Timer_CheckProgressTimeout
@onready var debug_chronometer := Chronometer.new()


func _ready():
	last_activity_time = Time.get_ticks_msec()
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
	Global.close_navbar.emit()
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
		last_activity_time = Time.get_ticks_msec()
	progress = new_progress

	loading_progress_label.text = "LOADING %d%%" % floor(progress)
	var tween = get_tree().create_tween()
	var new_width = loading_progress.get_parent().size.x * (progress / 100.0)
	tween.tween_property(loading_progress, "position:x", new_width, 0.1)


func _on_timer_auto_move_carousel_timeout():
	next_page()


func _on_timer_check_progress_timeout_timeout():
	if Global.scene_runner.is_paused():
		last_activity_time = Time.get_ticks_msec()
		return

	var loading_resources = (
		Global.content_provider.count_loading_resources() - loaded_resources_offset
	)
	var loaded_resources = (
		Global.content_provider.count_loaded_resources() - loaded_resources_offset
	)
	var download_speed_mbs: float = Global.content_provider.get_download_speed_mbs()

	# Update activity time if downloads are happening (resources being loaded)
	# This prevents timeout from triggering while actual work is in progress
	var is_actively_downloading = download_speed_mbs > 0.01 or loading_resources > loaded_resources
	if is_actively_downloading:
		last_activity_time = Time.get_ticks_msec()

	label_loading_state.text = (
		"(%d/%d resources at %.2fmb/s)" % [loaded_resources, loading_resources, download_speed_mbs]
	)

	var inactive_seconds: int = int(floor((Time.get_ticks_msec() - last_activity_time) / 1000.0))
	if inactive_seconds > 20:
		# Skip showing modal during tests to avoid affecting screenshots
		if Global.testing_scene_mode or Global.cli.scene_test_mode:
			return
		Global.modal_manager.async_show_scene_timeout_modal()
		# LOADING_TIMEOUT metric
		var timeout_data = {
			"loaded_resources": loaded_resources,
			"loading_resources": loading_resources,
			"download_speed_mbs": download_speed_mbs,
			"scene_id": str(Global.scene_fetcher.current_scene_entity_id),
			"inactive_seconds": inactive_seconds
		}
		Global.metrics.track_screen_viewed("LOADING_TIMEOUT", JSON.stringify(timeout_data))

		timer_check_progress_timeout.stop()


func _on_loading_screen_progress_logic_loading_show_requested():
	last_activity_time = Time.get_ticks_msec()
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
