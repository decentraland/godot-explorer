extends Control

## ExoPlayer Stress Test Scene
## Tests multiple video players with different formats, resolutions, and live streams
## Portrait mode layout with minimal controls per video

# Test video sources - diverse formats and resolutions
const TEST_VIDEOS: Array[Dictionary] = [
	{
		"name": "Big Buck Bunny (1080p)",
		"url": "https://commondatastorage.googleapis.com/gtv-videos-bucket/sample/BigBuckBunny.mp4",
		"type": "VOD"
	},
	{
		"name": "Elephants Dream (720p)",
		"url":
		"https://commondatastorage.googleapis.com/gtv-videos-bucket/sample/ElephantsDream.mp4",
		"type": "VOD"
	},
	{
		"name": "Sintel (480p)",
		"url": "https://commondatastorage.googleapis.com/gtv-videos-bucket/sample/Sintel.mp4",
		"type": "VOD"
	},
	{
		"name": "Tears of Steel (4K)",
		"url": "https://commondatastorage.googleapis.com/gtv-videos-bucket/sample/TearsOfSteel.mp4",
		"type": "VOD"
	},
	{
		"name": "HLS Live Test",
		"url": "https://test-streams.mux.dev/x36xhzz/x36xhzz.m3u8",
		"type": "HLS"
	},
	{
		"name": "DASH Test",
		"url": "https://dash.akamaized.net/akamai/bbb_30fps/bbb_30fps.mpd",
		"type": "DASH"
	},
]

# Player instances
var players: Array[ExoPlayer] = []
var player_panels: Array[Control] = []

# Performance monitor
var perf_monitor: PerformanceMonitor = null

# Grid layout config
var grid_columns: int = 2
var visible_players: int = 4  # How many players to show at once

# Scroll state
var scroll_offset: int = 0

@onready var grid_container: GridContainer = $VBoxContainer/ScrollContainer/GridContainer
@onready var info_label: Label = $VBoxContainer/InfoLabel
@onready var scroll_container: ScrollContainer = $VBoxContainer/ScrollContainer


func _ready():
	# Set up performance monitor
	perf_monitor = PerformanceMonitor.new()
	add_child(perf_monitor)

	# Adjust grid columns for portrait mode
	grid_container.columns = grid_columns

	# Create initial players
	_create_players()

	# Update info
	_update_info()


func _process(_delta):
	# Update all player textures
	for player in players:
		if player and is_instance_valid(player):
			player.update_texture()


func _create_players():
	for i in range(min(visible_players, TEST_VIDEOS.size())):
		_create_player_panel(i)


func _create_player_panel(video_index: int) -> Control:
	var video_data = TEST_VIDEOS[video_index]

	# Create panel container
	var panel = PanelContainer.new()
	panel.custom_minimum_size = Vector2(0, 280)
	panel.size_flags_horizontal = Control.SIZE_EXPAND_FILL

	var vbox = VBoxContainer.new()
	panel.add_child(vbox)

	# Header with name and type badge
	var header = HBoxContainer.new()
	vbox.add_child(header)

	var name_label = Label.new()
	name_label.text = video_data["name"]
	name_label.add_theme_font_size_override("font_size", 12)
	name_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	header.add_child(name_label)

	var type_label = Label.new()
	type_label.text = "[%s]" % video_data["type"]
	type_label.add_theme_font_size_override("font_size", 10)
	type_label.add_theme_color_override("font_color", _get_type_color(video_data["type"]))
	header.add_child(type_label)

	# Video display
	var video_rect = TextureRect.new()
	video_rect.name = "VideoRect"
	video_rect.custom_minimum_size = Vector2(0, 160)
	video_rect.size_flags_vertical = Control.SIZE_EXPAND_FILL
	video_rect.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
	video_rect.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_CENTERED
	vbox.add_child(video_rect)

	# Status label
	var status_label = Label.new()
	status_label.name = "StatusLabel"
	status_label.text = "Not loaded"
	status_label.add_theme_font_size_override("font_size", 10)
	status_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	vbox.add_child(status_label)

	# Minimal controls
	var controls = HBoxContainer.new()
	controls.alignment = BoxContainer.ALIGNMENT_CENTER
	vbox.add_child(controls)

	var load_btn = Button.new()
	load_btn.text = "Load"
	load_btn.custom_minimum_size = Vector2(50, 0)
	load_btn.pressed.connect(_on_load_pressed.bind(video_index))
	controls.add_child(load_btn)

	var play_btn = Button.new()
	play_btn.text = ">"
	play_btn.custom_minimum_size = Vector2(30, 0)
	play_btn.pressed.connect(_on_play_pressed.bind(video_index))
	controls.add_child(play_btn)

	var pause_btn = Button.new()
	pause_btn.text = "||"
	pause_btn.custom_minimum_size = Vector2(30, 0)
	pause_btn.pressed.connect(_on_pause_pressed.bind(video_index))
	controls.add_child(pause_btn)

	var stop_btn = Button.new()
	stop_btn.text = "X"
	stop_btn.custom_minimum_size = Vector2(30, 0)
	stop_btn.pressed.connect(_on_stop_pressed.bind(video_index))
	controls.add_child(stop_btn)

	grid_container.add_child(panel)
	player_panels.append(panel)

	# Create ExoPlayer
	var player = ExoPlayer.new()
	add_child(player)
	players.append(player)

	# Initialize texture
	player.init_texture(640, 360)
	video_rect.texture = player.get_texture()

	return panel


func _get_type_color(type: String) -> Color:
	match type:
		"VOD":
			return Color.GREEN
		"HLS":
			return Color.CYAN
		"DASH":
			return Color.ORANGE
		"LIVE":
			return Color.RED
		_:
			return Color.WHITE


func _on_load_pressed(index: int):
	if index >= players.size():
		return

	var player = players[index]
	var video_data = TEST_VIDEOS[index]

	player.set_source_url(video_data["url"])
	_update_status(index, "Loading...")


func _on_play_pressed(index: int):
	if index >= players.size():
		return

	players[index].play()
	_update_status(index, "Playing")


func _on_pause_pressed(index: int):
	if index >= players.size():
		return

	players[index].pause()
	_update_status(index, "Paused")


func _on_stop_pressed(index: int):
	if index >= players.size():
		return

	players[index].stop()
	_update_status(index, "Stopped")


func _update_status(index: int, status: String):
	if index >= player_panels.size():
		return

	var panel = player_panels[index]
	var status_label = panel.get_node("VBoxContainer/StatusLabel") as Label
	if status_label:
		var player = players[index]
		var mode = "GPU" if player.is_gpu_mode else "CPU"
		var size = "%dx%d" % [player.video_width, player.video_height]
		status_label.text = "%s [%s] %s" % [status, mode, size]


func _update_info():
	var total_players = players.size()
	var playing_count = 0
	var gpu_count = 0

	for player in players:
		if player.is_playing():
			playing_count += 1
		if player.is_gpu_mode:
			gpu_count += 1

	info_label.text = (
		"Players: %d | Playing: %d | GPU Mode: %d" % [total_players, playing_count, gpu_count]
	)


func _on_load_all_pressed():
	for i in range(players.size()):
		_on_load_pressed(i)


func _on_play_all_pressed():
	for i in range(players.size()):
		_on_play_pressed(i)


func _on_pause_all_pressed():
	for i in range(players.size()):
		_on_pause_pressed(i)


func _on_stop_all_pressed():
	for i in range(players.size()):
		_on_stop_pressed(i)


func _on_add_player_pressed():
	var next_index = players.size()
	if next_index < TEST_VIDEOS.size():
		_create_player_panel(next_index)
		_update_info()


func _on_remove_player_pressed():
	if players.size() > 1:
		var _last_index = players.size() - 1

		# Remove player
		var player = players.pop_back()
		if player:
			player.queue_free()

		# Remove panel
		var panel = player_panels.pop_back()
		if panel:
			panel.queue_free()

		_update_info()


func _exit_tree():
	for player in players:
		if player:
			player.queue_free()
