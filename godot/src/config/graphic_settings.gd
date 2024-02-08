class_name GraphicSettings extends RefCounted


static func connect_global_signal(root: Window):
	root.size_changed.connect(GraphicSettings.apply_ui_zoom.bind(root))


static func get_max_ui_zoom(root: Window) -> float:
	var screen_size = root.size

	# Should it matter if it's on Landscape or portrait?
	# TODO: for now, there is no portrait support
	# var orientation = DisplayServer.screen_get_orientation(root.current_screen)

	var x_factor: float = screen_size.x / 1280.0
	var y_factor: float = screen_size.y / 720.0
	var factor_limit: float = max(min(x_factor, y_factor), 1.0)

	return factor_limit


static func get_ui_zoom_available(root: Window) -> Dictionary:
	var max_zoom := get_max_ui_zoom(root)
	var ret: Dictionary = {}
	ret["75%"] = 0.75

	var zoom := 1.0
	while zoom < max_zoom:
		var key: String = str(int(zoom * 100.0)) + "%"
		ret[key] = zoom
		zoom += 0.25

	ret["Max"] = max_zoom
	return ret


static func apply_ui_zoom(root: Window):
	var factor = max(0.75, min(get_max_ui_zoom(root), Global.config.ui_zoom))

	if Global.config.ui_zoom < 0.0:
		var dpi := DisplayServer.screen_get_dpi()

		if dpi < 120:
			factor = 1.0
		elif dpi < 240:
			factor = 1.5
		else:
			factor = 2.0

		factor = max(0.75, min(get_max_ui_zoom(root), factor))
		Global.config.ui_zoom = factor

	root.content_scale_factor = factor


static func apply_window_config() -> void:
	if Global.is_mobile():
		return

	if Global.config.windowed:
		DisplayServer.window_set_mode(DisplayServer.WINDOW_MODE_WINDOWED)
	else:
		DisplayServer.window_set_mode(DisplayServer.WINDOW_MODE_FULLSCREEN)


static func apply_fps_limit():
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
