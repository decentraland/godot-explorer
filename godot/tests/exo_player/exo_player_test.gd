extends Control

## ExoPlayer Test Scene
## This scene demonstrates how to use the ExoPlayer wrapper for video playback on Android

# ExoPlayer instance
var player: ExoPlayer = null

# Performance monitor
var perf_monitor: PerformanceMonitor = null

# Update state
var is_updating_slider: bool = false

@onready var video_rect: TextureRect = $VBoxContainer/VideoRect
@onready var url_line_edit: LineEdit = $VBoxContainer/URLInput/URLLineEdit
@onready var path_line_edit: LineEdit = $VBoxContainer/LocalInput/PathLineEdit
@onready var position_slider: HSlider = $VBoxContainer/Slider/PositionSlider
@onready var time_label: Label = $VBoxContainer/Slider/TimeLabel
@onready var volume_slider: HSlider = $VBoxContainer/Volume/VolumeSlider
@onready var volume_label: Label = $VBoxContainer/Volume/VolumeLabel
@onready var loop_check_box: CheckBox = $VBoxContainer/Options/LoopCheckBox
@onready var info_label: Label = $VBoxContainer/InfoLabel


func _ready():
	# Create and initialize the ExoPlayer
	player = ExoPlayer.new()
	add_child(player)

	# Initialize texture with placeholder size (will be resized when video loads)
	player.init_texture(100, 100)

	# Set the texture on the video rect immediately
	var texture = player.get_texture()
	if texture:
		video_rect.texture = texture

	# Set default test URL (Big Buck Bunny - free test video)
	# Note: Must use HTTPS on Android due to cleartext traffic restrictions
	url_line_edit.text = "https://commondatastorage.googleapis.com/gtv-videos-bucket/sample/BigBuckBunny.mp4"

	# Show platform info
	if OS.get_name() == "Android":
		info_label.text = "Running on Android - ExoPlayer available"
	else:
		info_label.text = "Not running on Android - ExoPlayer not available (stub mode)"

	# Add performance monitor overlay
	perf_monitor = PerformanceMonitor.new()
	add_child(perf_monitor)
	perf_monitor.set_video_player(player)


func _process(_delta):
	if not player:
		return

	# Update UI with player state when playing
	if player.is_playing():
		# Update position slider (block signal to prevent seek feedback loop)
		if not is_updating_slider:
			var duration = player.get_duration()
			if duration > 0:
				var current_pos = player.get_position()
				position_slider.max_value = duration
				# Block the signal while updating to prevent seek feedback loop
				position_slider.set_value_no_signal(current_pos)

				# Update time label
				time_label.text = "%s / %s" % [_format_time(current_pos), _format_time(duration)]


func _format_time(seconds: float) -> String:
	var mins = int(seconds) / 60
	var secs = int(seconds) % 60
	return "%02d:%02d" % [mins, secs]


func _on_play_button_pressed():
	if player:
		player.play()
		info_label.text = "Playing..."


func _on_pause_button_pressed():
	if player:
		player.pause()
		info_label.text = "Paused"


func _on_stop_button_pressed():
	if player:
		player.stop()
		info_label.text = "Stopped"


func _on_load_button_pressed():
	if player:
		var url = url_line_edit.text
		if url.is_empty():
			info_label.text = "Please enter a URL"
			return

		info_label.text = "Loading URL: " + url
		var success = player.set_source_url(url)
		if success:
			info_label.text = "URL loaded, preparing..."
		else:
			info_label.text = "Failed to load URL"


func _on_load_local_button_pressed():
	if player:
		var path = path_line_edit.text
		if path.is_empty():
			info_label.text = "Please enter a file path"
			return

		info_label.text = "Loading local file: " + path
		var success = player.set_source_local(path)
		if success:
			info_label.text = "Local file loaded, preparing..."
		else:
			info_label.text = "Failed to load local file"


# gdlint:ignore = async-function-name
func _on_position_slider_value_changed(value: float):
	if player and not is_updating_slider:
		is_updating_slider = true
		player.set_position(value)
		# Small delay before allowing slider to update again
		await get_tree().create_timer(0.1).timeout
		is_updating_slider = false


func _on_volume_slider_value_changed(value: float):
	if player:
		player.set_volume(value)
		volume_label.text = "%d%%" % int(value * 100)


func _on_loop_check_box_toggled(toggled_on: bool):
	if player:
		player.set_looping(toggled_on)
		info_label.text = "Looping: " + ("On" if toggled_on else "Off")


func _on_info_button_pressed():
	if player:
		info_label.text = player.get_player_info()


func _exit_tree():
	# Clean up
	if player:
		player.queue_free()
