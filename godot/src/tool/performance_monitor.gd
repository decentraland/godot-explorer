class_name PerformanceMonitor
extends CanvasLayer

## Real-time performance monitoring overlay for debugging
## Shows FPS, frame time, memory usage, and video-specific metrics

@export var enabled: bool = true:
	set(value):
		enabled = value
		visible = value

@export var update_interval: float = 0.5  # Update every 0.5 seconds

var label: Label
var update_timer: float = 0.0

# Frame time tracking
var frame_times: Array[float] = []
var max_frame_samples: int = 60

# Video player reference (optional)
var video_player: ExoPlayer = null


func _ready():
	layer = 100  # Draw on top of everything

	label = Label.new()
	label.position = Vector2(10, 10)
	label.add_theme_font_size_override("font_size", 14)
	label.add_theme_color_override("font_color", Color.WHITE)
	label.add_theme_color_override("font_shadow_color", Color.BLACK)
	label.add_theme_constant_override("shadow_offset_x", 1)
	label.add_theme_constant_override("shadow_offset_y", 1)

	# Add background for readability
	var bg = ColorRect.new()
	bg.color = Color(0, 0, 0, 0.5)
	bg.custom_minimum_size = Vector2(300, 200)
	bg.mouse_filter = Control.MOUSE_FILTER_IGNORE
	add_child(bg)

	label.mouse_filter = Control.MOUSE_FILTER_IGNORE
	add_child(label)

	visible = enabled


func _process(delta: float):
	if not enabled:
		return

	# Track frame times
	frame_times.append(delta * 1000.0)  # Convert to ms
	if frame_times.size() > max_frame_samples:
		frame_times.pop_front()

	update_timer += delta
	if update_timer >= update_interval:
		update_timer = 0.0
		_update_display()


func _update_display():
	var text := ""

	# FPS and Frame Time
	var fps := Performance.get_monitor(Performance.TIME_FPS)
	var avg_frame_time := 0.0
	var max_frame_time := 0.0
	if frame_times.size() > 0:
		for ft in frame_times:
			avg_frame_time += ft
			max_frame_time = max(max_frame_time, ft)
		avg_frame_time /= frame_times.size()

	text += "=== PERFORMANCE ===\n"
	text += "FPS: %.1f\n" % fps
	text += "Frame: %.2f ms (avg) / %.2f ms (max)\n" % [avg_frame_time, max_frame_time]

	# Process time breakdown
	var process_time := Performance.get_monitor(Performance.TIME_PROCESS) * 1000.0
	var _physics_time := Performance.get_monitor(Performance.TIME_PHYSICS_PROCESS) * 1000.0
	var _navigation_time := Performance.get_monitor(Performance.TIME_NAVIGATION_PROCESS) * 1000.0
	text += "Process: %.2f ms\n" % process_time

	# Memory
	text += "\n=== MEMORY ===\n"
	var static_mem := Performance.get_monitor(Performance.MEMORY_STATIC) / 1048576.0  # MB
	var static_max := Performance.get_monitor(Performance.MEMORY_STATIC_MAX) / 1048576.0
	text += "Static: %.1f MB (max: %.1f MB)\n" % [static_mem, static_max]

	# Object counts
	text += "\n=== OBJECTS ===\n"
	var object_count := Performance.get_monitor(Performance.OBJECT_COUNT)
	var resource_count := Performance.get_monitor(Performance.OBJECT_RESOURCE_COUNT)
	var node_count := Performance.get_monitor(Performance.OBJECT_NODE_COUNT)
	text += "Objects: %d\n" % object_count
	text += "Resources: %d\n" % resource_count
	text += "Nodes: %d\n" % node_count

	# Rendering
	text += "\n=== RENDERING ===\n"
	var draw_calls := Performance.get_monitor(Performance.RENDER_TOTAL_DRAW_CALLS_IN_FRAME)
	var objects_drawn := Performance.get_monitor(Performance.RENDER_TOTAL_OBJECTS_IN_FRAME)
	text += "Draw calls: %d\n" % draw_calls
	text += "Objects drawn: %d\n" % objects_drawn

	# Video player specific (if set)
	if video_player and is_instance_valid(video_player):
		text += "\n=== VIDEO PLAYER ===\n"
		text += "Mode: %s\n" % ("GPU" if video_player.is_gpu_mode else "CPU")
		text += "Size: %dx%d\n" % [video_player.video_width, video_player.video_height]
		text += "Playing: %s\n" % str(video_player.is_playing())
		text += (
			"Position: %.1f / %.1f s\n" % [video_player.get_position(), video_player.get_duration()]
		)

	# Android specific memory (if available)
	if OS.get_name() == "Android" and Engine.has_singleton("dcl-godot-android"):
		var plugin = Engine.get_singleton("dcl-godot-android")
		text += "\n" + plugin.getMemorySummary()

	label.text = text

	# Resize background to fit text
	var bg = get_child(0) as ColorRect
	if bg:
		bg.custom_minimum_size = label.get_minimum_size() + Vector2(20, 20)


## Set a video player to monitor
func set_video_player(player: ExoPlayer):
	video_player = player


## Get current metrics as a dictionary (for logging)
func get_metrics() -> Dictionary:
	return {
		"fps": Performance.get_monitor(Performance.TIME_FPS),
		"frame_time_avg": _get_avg_frame_time(),
		"frame_time_max": _get_max_frame_time(),
		"memory_static_mb": Performance.get_monitor(Performance.MEMORY_STATIC) / 1048576.0,
		"object_count": Performance.get_monitor(Performance.OBJECT_COUNT),
		"draw_calls": Performance.get_monitor(Performance.RENDER_TOTAL_DRAW_CALLS_IN_FRAME),
	}


func _get_avg_frame_time() -> float:
	if frame_times.is_empty():
		return 0.0
	var sum := 0.0
	for ft in frame_times:
		sum += ft
	return sum / frame_times.size()


func _get_max_frame_time() -> float:
	if frame_times.is_empty():
		return 0.0
	var max_val := 0.0
	for ft in frame_times:
		max_val = max(max_val, ft)
	return max_val
