class_name ResolutionManager extends RefCounted

var resolutions_16_9 := [
	Vector2i(1280, 720),
	Vector2i(1366, 768),
	Vector2i(1920, 1080),
	Vector2i(2560, 1440),
	Vector2i(3200, 1800),
	Vector2i(3840, 2160)
]

var window_options = {}
var resolution_options = {}


func refresh_window_options():
	window_options.clear()

	var screen_size = DisplayServer.screen_get_size()
	for j in range(resolutions_16_9.size()):
		var option_size = resolutions_16_9[j]
		if screen_size.x >= option_size.x and screen_size.y >= option_size.y:
			var key: String = "%d x %d" % [option_size.x, option_size.y]
			window_options[key] = option_size

	refresh_resolutions()


func refresh_resolutions():
	resolution_options.clear()

	var window_size = DisplayServer.window_get_size()
	for j in range(resolutions_16_9.size()):
		var res_size = resolutions_16_9[j]
		if window_size.x >= res_size.x and window_size.y >= res_size.y:
			var key: String = "%d x %d" % [res_size.x, res_size.y]
			resolution_options[key] = res_size


func change_window_size(window: Window, viewport: Viewport, option_key: String) -> void:
	if not window_options.has(option_key):
		return

	DisplayServer.window_set_size(window_options[option_key])
	viewport.size = Vector2(window_options[option_key])
	Global.config

	change_resolution(window, viewport, option_key)

	refresh_resolutions()


func change_resolution(window: Window, viewport: Viewport, option_key: String) -> void:
	if not window_options.has(option_key):
		return

	DisplayServer.get_window_list()
	var res_size = Vector2(window_options[option_key])
	var factor = viewport.size.x / res_size.x
	if factor == 1.0:
		window.content_scale_mode = Window.CONTENT_SCALE_MODE_DISABLED
		window.content_scale_size = Vector2.ZERO
	else:
		window.content_scale_mode = Window.CONTENT_SCALE_MODE_VIEWPORT
		window.content_scale_size = res_size


func change_ui_scale(window: Window, value: float):
	window.content_scale_factor = value


func center_window(window: Window):
	var screen_size = DisplayServer.screen_get_size()
	DisplayServer.window_set_position(screen_size * 0.5 - window.size * 0.5)


func apply_fps_limit():
	match Global.config.limit_fps:
		0:  # VSync
			Engine.max_fps = 0
			DisplayServer.window_set_vsync_mode(DisplayServer.VSYNC_ENABLED)
		1:  # No Limit
			Engine.max_fps = 0
			DisplayServer.window_set_vsync_mode(DisplayServer.VSYNC_DISABLED)
		2:  # 30
			Engine.max_fps = 30
			DisplayServer.window_set_vsync_mode(DisplayServer.VSYNC_DISABLED)
		3:  # 60
			Engine.max_fps = 60
			DisplayServer.window_set_vsync_mode(DisplayServer.VSYNC_DISABLED)
		4:  # 120
			Engine.max_fps = 120
			DisplayServer.window_set_vsync_mode(DisplayServer.VSYNC_DISABLED)
