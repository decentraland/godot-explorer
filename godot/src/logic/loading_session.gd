class_name LoadingSession
extends Node
## The engine's side of a world load: content limits, comms and audio while it runs, the
## loading funnel metrics, the stall timeout, and the report that ends the core's Loading state.
## The screen the player sees is the shell's.

## Hard maximum loading time in seconds, whatever the download activity.
const MAX_LOADING_TIME_SECONDS := 90.0
const STALL_SECONDS := 20
const CHECK_INTERVAL_SECONDS := 1.0

var _running := false
var _start_msec := 0
var _last_activity_msec := 0
var _loaded_offset := 0
var _last_percent := 0.0
var _timeout_tracked := false
var _timer := Timer.new()


func _ready() -> void:
	_timer.wait_time = CHECK_INTERVAL_SECONDS
	_timer.timeout.connect(_on_check_timeout)
	add_child(_timer)
	var runner := Global.scene_runner
	runner.loading_started.connect(_on_scene_loading_started)
	runner.loading_progress.connect(_on_progress)
	runner.loading_complete.connect(_on_complete)
	runner.loading_timeout.connect(_on_session_timeout)


func is_running() -> bool:
	return _running


## `begin_episode` is false when the caller already opened the episode on the navigation intent;
## a second one would supersede it and lose what it was measuring.
func begin(intended_realm: String, when: String, begin_episode: bool = true) -> void:
	if begin_episode:
		Global.scene_runner.loading_begin_episode(
			when,
			intended_realm if not intended_realm.is_empty() else Global.realm.get_realm_string()
		)
	_start()
	if not when.is_empty():
		var data := {
			"position": str(Global.scene_fetcher.current_position),
			"realm":
			intended_realm if not intended_realm.is_empty() else Global.realm.get_realm_string(),
			"when": when
		}
		Global.metrics.track_screen_viewed("LOADING_START", JSON.stringify(data))


## The load is over without a scene: a realm change that failed, or the player backing out.
func end(status: String = "Success") -> void:
	if not _running:
		return
	_running = false
	_timer.stop()
	Global.content_provider.set_max_concurrent_downloads(12)
	Global.content_provider.set_max_low_priority_downloads(12)
	# Held back during the GP benchmark so two devices running the matrix never see each other.
	if not Global.is_gp_benchmark():
		Global.comms.release_comms()
	AudioSettings.apply_scene_volume_settings()
	AudioSettings.apply_voice_chat_volume_settings()
	AudioSettings.apply_avatar_and_emotes_volume_settings()
	Global.loading_finished.emit()
	Global.scene_runner.loading_end_episode("hidden")
	var pos := Global.scene_fetcher.current_position
	Global.metrics.track_screen_viewed(
		"LOADING_END",
		JSON.stringify(
			{
				"scene_id": Global.scene_fetcher.current_scene_entity_id,
				"position": "%d,%d" % [pos.x, pos.y],
				"status": status
			}
		)
	)
	var explorer := Global.get_explorer()
	if explorer != null:
		Global.engine.report_loading_finished(
			Global.realm.get_realm_string(), explorer.parcel_position.x, explorer.parcel_position.y
		)


## The player left while it was loading.
func cancel() -> void:
	if _running:
		_running = false
		_timer.stop()
		Global.scene_runner.loading_end_episode("cancelled")


func _start() -> void:
	var now := Time.get_ticks_msec()
	_last_activity_msec = now
	if not _running:
		_start_msec = now
		_loaded_offset = Global.content_provider.count_loaded_resources()
		_timeout_tracked = false
	_running = true
	_last_percent = 0.0
	_timer.start()
	Global.content_provider.set_max_concurrent_downloads(_max_downloads())
	Global.content_provider.set_max_low_priority_downloads(2)
	# The scene room connects once loading finishes.
	Global.comms.hold_comms()
	# No voice, scene or avatar SFX (footsteps, jump/land) while loading.
	AudioSettings.apply_scene_volume_settings(0.0)
	AudioSettings.apply_voice_chat_volume_settings(0.0)
	AudioSettings.apply_avatar_and_emotes_volume_settings(0.0)
	Global.loading_started.emit()
	Global.release_mouse()
	Global.engine.report_loading_progress(0.0)


## `max-downloads=<n>` (deeplink) overrides the default 12, for benchmarking the download stage.
func _max_downloads() -> int:
	if Global.deep_link_obj != null:
		var value := str(Global.deep_link_obj.params.get("max-downloads", ""))
		if value.is_valid_int() and value.to_int() > 0:
			return value.to_int()
	return 12


func _on_scene_loading_started(_session_id: int, _expected_count: int) -> void:
	_start()


func _on_progress(percent: float, _ready_count: int, _total_count: int) -> void:
	if not _running:
		return
	percent = clampf(percent, 0.0, 100.0)
	if percent != _last_percent:
		_last_activity_msec = Time.get_ticks_msec()
		_last_percent = percent
		Global.engine.report_loading_progress(percent / 100.0)


func _on_complete(_session_id: int) -> void:
	end()


func _on_session_timeout(_session_id: int) -> void:
	push_warning("Loading session timed out")
	end()


func _on_check_timeout() -> void:
	if not _running or _timeout_tracked:
		return
	if Global.scene_runner.is_paused():
		_last_activity_msec = Time.get_ticks_msec()
		return
	var speed: float = Global.content_provider.get_download_speed_mbs()
	# Only real network throughput counts as activity; a pending asset alone would defeat the
	# timeout on heavy scenes. Progress changes are counted in _on_progress.
	if speed > 0.01:
		_last_activity_msec = Time.get_ticks_msec()
	var elapsed := (Time.get_ticks_msec() - _start_msec) / 1000.0
	var inactive := int(floor((Time.get_ticks_msec() - _last_activity_msec) / 1000.0))
	if inactive <= STALL_SECONDS and elapsed <= MAX_LOADING_TIME_SECONDS:
		return
	# Tests and the GP benchmark have their own cap; a timeout would cut their sample short.
	if Global.testing_scene_mode or Global.cli.scene_test_mode or Global.is_gp_benchmark():
		return
	_timeout_tracked = true
	var loading := Global.content_provider.count_loading_resources() - _loaded_offset
	var loaded := Global.content_provider.count_loaded_resources() - _loaded_offset
	Global.metrics.track_screen_viewed(
		"LOADING_TIMEOUT",
		JSON.stringify(
			{
				"loaded_resources": loaded,
				"loading_resources": loading,
				"download_speed_mbs": speed,
				"scene_id": str(Global.scene_fetcher.current_scene_entity_id),
				"inactive_seconds": inactive,
				"total_elapsed_seconds": int(elapsed),
				"timeout_reason": "max_time" if elapsed > MAX_LOADING_TIME_SECONDS else "inactivity"
			}
		)
	)
