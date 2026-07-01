extends Control

## Hard maximum loading time in seconds. If loading takes longer than this,
## the "RUN ANYWAY" modal is shown regardless of download activity.
const MAX_LOADING_TIME_SECONDS := 90.0

var progress: float = 0.0
var last_activity_time := Time.get_ticks_msec()
var loading_start_time := 0
var last_hide_click := 0.0
var loaded_resources_offset := 0

var _place_data_set: bool = false
var _current_bg_url: String = ""
var _fetch_generation: int = 0

@onready var loading_progress_label: Label = %Label_LoadingProgress
@onready var label_loading_state: Label = %Label_LoadingState
@onready var texture_progress_bar: TextureProgressBar = %TextureProgressBar
@onready var rich_text_label_place_name: TrimmedRichTextLabel = %RichTextLabel_PlaceName
@onready var texture_rect_background: TextureRect = %TextureRect_Background
@onready var vbox_data: VBoxContainer = %VBoxContainer_Data
@onready var rich_text_label_creator: RichTextLabel = %RichTextLabel

@onready var loading_screen_progress_logic: Node = $LoadingScreenProgressLogic
@onready var timer_check_progress_timeout: Timer = $Timer_CheckProgressTimeout
@onready var debug_chronometer := Chronometer.new()

static var _low_spec_toast_shown: bool = false


func _ready() -> void:
	last_activity_time = Time.get_ticks_msec()
	Global.scene_runner.loading_started.connect(_on_scene_runner_loading_started)


func enable_loading_screen() -> void:
	if !debug_chronometer:
		debug_chronometer = Chronometer.new()
	debug_chronometer.restart("Starting to load scene")
	_clear_place_ui()
	Global.loading_started.emit()
	Global.release_mouse()
	loading_screen_progress_logic.enable_loading_screen()


func _clear_place_ui() -> void:
	_fetch_generation += 1
	_place_data_set = false
	_current_bg_url = ""
	texture_rect_background.texture = null
	texture_rect_background.modulate = Color.TRANSPARENT
	vbox_data.modulate = Color.TRANSPARENT
	rich_text_label_place_name.text = ""
	rich_text_label_creator.hide()


func async_hide_loading_screen_effect():
	_clear_place_ui()
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


func _on_scene_runner_loading_started(_session_id: int, _expected_count: int) -> void:
	if _place_data_set:
		return
	var pos = Global.scene_fetcher.current_position
	if pos == SceneFetcher.INVALID_PARCEL:
		return
	# Increment so any previous in-flight fetch for an earlier loading session is discarded.
	_fetch_generation += 1
	_async_fetch_place_data(pos, _fetch_generation)


func _async_fetch_place_data(pos: Vector2i, generation: int) -> void:
	# last_realm_joined is written before change_scene_to_file, so it reflects the intended
	# realm even when Global.realm.realm_url/realm_name is still stale (async_set_realm not done yet).
	var intended_realm = Global.get_config().last_realm_joined
	var result: Variant
	if intended_realm.is_empty() or Realm.is_genesis_city(intended_realm):
		result = await PlacesHelper.async_get_by_position(pos)
	else:
		result = await PlacesHelper.async_get_by_names(intended_realm)
	if not is_instance_valid(self) or not is_inside_tree() or generation != _fetch_generation:
		return
	if result is PromiseError:
		return
	var json: Dictionary = result.get_string_response_as_json()
	var data_array = json.get("data", [])
	if data_array.is_empty():
		return
	set_place_data(data_array[0])


func set_place_data(data: Dictionary) -> void:
	_place_data_set = true
	var title = data.get("title", "")
	var creator = data.get("contact_name", "")
	set_place_name(title if title is String else "")
	set_place_creator(creator if creator is String else "")
	var image_url = data.get("image", "")
	if image_url is String and not image_url.is_empty():
		set_place_image(image_url)
	var tween = create_tween()
	tween.tween_property(vbox_data, "modulate", Color.WHITE, 0.125)


func set_place_name(place_name: String) -> void:
	if place_name.is_empty():
		return
	rich_text_label_place_name.set_text_trimmed(place_name)


func set_place_creator(creator: String) -> void:
	if creator.is_empty():
		rich_text_label_creator.hide()
		return
	rich_text_label_creator.show()
	rich_text_label_creator.text = "[color=#DF9CFF]By[/color] " + creator


func set_place_image(image_url: String) -> void:
	if _current_bg_url == image_url:
		return
	_current_bg_url = image_url
	_async_set_background.call_deferred(image_url)


func _apply_background_texture(texture: Texture2D) -> void:
	texture_rect_background.texture = texture
	var tween = create_tween()
	tween.tween_property(texture_rect_background, "modulate", Color.WHITE, 0.125)


func _async_set_background(url: String) -> void:
	var url_hash := AsyncImage._get_hash_from_url(url)
	var promise = Global.content_provider.fetch_texture_by_url(url_hash, url)
	var result = await PromiseUtils.async_awaiter(promise)
	if not is_instance_valid(self) or not is_inside_tree():
		return
	if url != _current_bg_url:
		return
	if result is PromiseError or result.failed:
		return
	_apply_background_texture(result.texture)


func _on_close_button_pressed() -> void:
	_clear_place_ui()
	Global.return_to_discover()
