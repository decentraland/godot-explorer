extends Control

## Hard maximum loading time in seconds. If loading takes longer than this,
## the "RUN ANYWAY" modal is shown regardless of download activity.
const MAX_LOADING_TIME_SECONDS := 90.0

var progress: float = 0.0
var last_activity_time := Time.get_ticks_msec()
var loading_start_time := 0
var popup_warning_pos_y: int = 0

var last_hide_click := 0.0

var loaded_resources_offset := 0

@onready var loading_progress = %ColorRect_LoadingProgress
@onready var loading_progress_label = %Label_LoadingProgress
@onready var label_loading_state = %Label_LoadingState

@onready
var carousel = $VBox_Loading/ColorRect_Background/Control_Discover/VBoxContainer/HBoxContainer_Content/CarouselViewport/SubViewport/Carousel

@onready var timer_auto_move_carousel = $Timer_AutoMoveCarousel

@onready var loading_screen_progress_logic = $LoadingScreenProgressLogic
@onready var timer_check_progress_timeout = $Timer_CheckProgressTimeout
@onready var debug_chronometer := Chronometer.new()
@onready var texture_progress_bar: TextureProgressBar = %TextureProgressBar

static var _low_spec_toast_shown: bool = false


func _ready():
	last_activity_time = Time.get_ticks_msec()

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
	Global.close_chat.emit()
	debug_chronometer.lap("Finished loading scene")
	Global.loading_finished.emit()
	timer_check_progress_timeout.stop()
	_low_spec_toast_shown = false
	var tween = get_tree().create_tween()
	modulate = Color.WHITE
	tween.tween_property(self, "modulate", Color.TRANSPARENT, 1.0)
	await tween.finished
	hide()
	modulate = Color.WHITE
	self.position.y = 0

	if Global.cli.measure_perf:
		var counter = (
			load("res://addons/dcl_dev_tools/dev_tools/resource_counter/resource_counter.gd").new()
		)
		add_child(counter)
		await get_tree().create_timer(5).timeout
		counter.log_active_counts()


func set_progress(new_progress: float):
	new_progress = clampf(new_progress, 0.0, 100.0)
	if progress != new_progress:
		last_activity_time = Time.get_ticks_msec()
	progress = new_progress

	loading_progress_label.text = "%d%%" % floor(progress)
	texture_progress_bar.value = progress


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

	# Update activity time only if there is actual network throughput.
	# Previously `loading_resources > loaded_resources` kept the timer alive
	# as long as ANY asset was pending, defeating the timeout on heavy scenes.
	# Progress value changes already reset last_activity_time via set_progress().
	if download_speed_mbs > 0.01:
		last_activity_time = Time.get_ticks_msec()

	var opt_suffix = ""
	if not DclGlobal.is_production() and Global.content_provider.get_optimized_scene_count() > 0:
		opt_suffix = " - Opt"
	label_loading_state.text = (
		"(%d/%d resources at %.2fmb/s)%s"
		% [loaded_resources, loading_resources, download_speed_mbs, opt_suffix]
	)

	# Absolute maximum loading time — never wait longer than this
	var total_elapsed_seconds = (Time.get_ticks_msec() - loading_start_time) / 1000.0
	var inactive_seconds: int = int(floor((Time.get_ticks_msec() - last_activity_time) / 1000.0))
	var should_timeout = inactive_seconds > 20 or total_elapsed_seconds > MAX_LOADING_TIME_SECONDS

	if should_timeout:
		# Skip showing modal during tests to avoid affecting screenshots
		# and during the GP benchmark, where the runner has its own 30 min
		# hard cap and the modal would force a half-loaded sample.
		if Global.testing_scene_mode or Global.cli.scene_test_mode or Global.is_gp_benchmark():
			return
		Global.modal_manager.async_show_scene_timeout_modal()
		# LOADING_TIMEOUT metric
		var timeout_reason = (
			"max_time" if total_elapsed_seconds > MAX_LOADING_TIME_SECONDS else "inactivity"
		)
		var timeout_data = {
			"loaded_resources": loaded_resources,
			"loading_resources": loading_resources,
			"download_speed_mbs": download_speed_mbs,
			"scene_id": str(Global.scene_fetcher.current_scene_entity_id),
			"inactive_seconds": inactive_seconds,
			"total_elapsed_seconds": int(total_elapsed_seconds),
			"timeout_reason": timeout_reason
		}
		Global.metrics.track_screen_viewed("LOADING_TIMEOUT", JSON.stringify(timeout_data))

		timer_check_progress_timeout.stop()


func _on_loading_screen_progress_logic_loading_show_requested():
	var now = Time.get_ticks_msec()
	last_activity_time = now
	loading_start_time = now
	timer_check_progress_timeout.start()
	loaded_resources_offset = Global.content_provider.count_loaded_resources()
	_show_low_spec_toast_if_needed()


func _show_low_spec_toast_if_needed():
	if _low_spec_toast_shown:
		return
	var deeplink_warning = Global.deep_link_obj and Global.deep_link_obj.low_spec_warning
	var is_low_spec = (
		Global.cli.low_spec_warning
		or deeplink_warning
		or (DclIosPlugin.is_available() and DclIosPlugin.is_low_spec_iphone())
	)
	if not is_low_spec:
		return

	_low_spec_toast_shown = true
	var toast_scene = load("res://src/ui/components/organisms/notifications/low_spec_toast.tscn")
	var toast = toast_scene.instantiate()
	add_child(toast)
	toast.async_show()


func _on_texture_rect_logo_gui_input(event: InputEvent) -> void:
	if event is InputEventScreenTouch:
		if event.pressed:
			var elapsed_time = Time.get_ticks_msec() - last_hide_click
			if elapsed_time <= 500:
				loading_screen_progress_logic.hide_loading_screen()
			last_hide_click = Time.get_ticks_msec()
