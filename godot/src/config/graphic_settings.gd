class_name GraphicSettings extends RefCounted


static func connect_global_signal(root: Window):
	root.size_changed.connect(GraphicSettings.apply_ui_zoom.bind(root))


static func get_max_ui_zoom(root: Window) -> float:
	var screen_size: Vector2 = root.size

	var base_resolution: Vector2
	if screen_size.x >= screen_size.y:
		# Landscape orientation base (minimum recommended)
		base_resolution = Vector2(1280, 720)
	else:
		# Portrait orientation base (minimum recommended)
		base_resolution = Vector2(720, 1280)

	var x_factor: float = screen_size.x / base_resolution.x
	var y_factor: float = screen_size.y / base_resolution.y

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


# Simple DPI-based scaling without aggressive resolution clamp
static func apply_ui_zoom(root: Window):
	pass
	#var dpi := DisplayServer.screen_get_dpi()
	#prints("Detected DPI:", dpi)
#
	#var factor: float
	##if Global.get_config().ui_zoom < 0.0:
	#if dpi >= 400:
		#factor = 3.0
	#elif dpi >= 300:
		#factor = 2.5
	#elif dpi >= 240:
		#factor = 2.0
	#elif dpi >= 120:
		#factor = 1.5
	#else:
		#factor = 1.0
	#Global.get_config().ui_zoom = factor
	##else:
	##	factor = Global.get_config().ui_zoom
	#
	## Optional minimum clamp only
	#factor = max(factor, 0.75)
	#root.content_scale_factor = factor
	#prints("Applied factor:", factor)



static func apply_window_config() -> void:
	if Global.is_mobile():
		return

	match Global.get_config().window_mode:
		0:  # Windowed
			DisplayServer.window_set_mode(DisplayServer.WINDOW_MODE_WINDOWED)
		1:  # Borderless
			DisplayServer.window_set_mode(DisplayServer.WINDOW_MODE_FULLSCREEN)
		2:  # Full screen
			DisplayServer.window_set_mode(DisplayServer.WINDOW_MODE_EXCLUSIVE_FULLSCREEN)


static func apply_fps_limit():
	match Global.get_config().limit_fps:
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
