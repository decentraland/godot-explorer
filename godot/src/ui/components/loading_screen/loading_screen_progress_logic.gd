extends Node
## Loading screen controller that listens to SceneManager loading signals.
## Uses the new Rust-based LoadingSession for accurate, monotonically-increasing progress.

signal loading_show_requested

@export var loading_screen: Control


func _ready():
	# Connect to SceneManager loading signals
	Global.scene_runner.loading_started.connect(_on_loading_started)
	Global.scene_runner.loading_phase_changed.connect(_on_phase_changed)
	Global.scene_runner.loading_progress.connect(_on_loading_progress)
	Global.scene_runner.loading_complete.connect(_on_loading_complete)
	Global.scene_runner.loading_timeout.connect(_on_loading_timeout)
	Global.scene_runner.loading_cancelled.connect(_on_loading_cancelled)


## Called by loading_screen.gd when it wants to enable loading
func enable_loading_screen():
	# Show loading screen immediately - the LoadingSession will update progress later
	Global.content_provider.set_max_concurrent_downloads(6)

	# Mute voice chat and scene volume during loading
	AudioSettings.apply_scene_volume_settings(0.0)
	AudioSettings.apply_voice_chat_volume_settings(0.0)

	loading_screen.show()
	loading_screen.set_progress(0)
	loading_show_requested.emit()


## Called by loading_screen.gd or popup button to force hide loading
func hide_loading_screen():
	_hide_loading_screen()


func _on_loading_started(_session_id: int, _expected_count: int):
	Global.content_provider.set_max_concurrent_downloads(6)

	# Mute voice chat and scene volume during loading
	AudioSettings.apply_scene_volume_settings(0.0)
	AudioSettings.apply_voice_chat_volume_settings(0.0)

	loading_screen.show()
	loading_screen.set_progress(0)
	loading_show_requested.emit()


func _on_phase_changed(_phase: String):
	# Could update status text here if loading screen supports it
	# e.g., loading_screen.set_status({"metadata": "Fetching scenes...", ...}.get(phase, "Loading..."))
	pass


func _on_loading_progress(percent: float, _ready_count: int, _total_count: int):
	loading_screen.set_progress(percent)


func _on_loading_complete(_session_id: int):
	loading_screen.set_progress(100)
	_hide_loading_screen()


func _on_loading_timeout(_session_id: int):
	push_warning("Loading session timed out")
	_hide_loading_screen()


func _on_loading_cancelled(_session_id: int):
	# A new loading session is about to start - keep the loading screen visible
	# The new session's loading_started signal will reset progress
	pass


func _hide_loading_screen():
	Global.content_provider.set_max_concurrent_downloads(6)

	# Restore voice chat and scene volume
	AudioSettings.apply_scene_volume_settings()
	AudioSettings.apply_voice_chat_volume_settings()

	loading_screen.async_hide_loading_screen_effect()

	# LOADING_END (Success) metric
	var end_data = {
		"scene_id": Global.scene_fetcher.current_scene_entity_id,
		"position": "%d,%d" % Global.scene_fetcher.current_position,
		"status": "Success"
	}
	Global.metrics.track_screen_viewed("LOADING_END", JSON.stringify(end_data))
