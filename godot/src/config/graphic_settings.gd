class_name GraphicSettings extends RefCounted


static func connect_global_signal(root: Window):
	root.size_changed.connect(GraphicSettings.apply_ui_zoom.bind(root))


static func get_max_ui_zoom(root: Window) -> float:
	var screen_size: Vector2 = root.size

	var base_resolution: Vector2
	base_resolution = Vector2(720, 720)

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
	var screen_size: Vector2 = root.size
	var base_resolution: Vector2 = Vector2(720, 720)
	var scale_x = screen_size.x / base_resolution.x
	var scale_y = screen_size.y / base_resolution.y

	# Choose the smaller scale to ensure content always fits on screen
	var scale = min(scale_x, scale_y)
	root.content_scale_factor = scale


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
	# Physics FPS matches render FPS but capped at 60 (default 30)
	var physics_fps := 30
	match Global.get_config().limit_fps:
		ConfigData.FpsLimitMode.VSYNC:
			Engine.max_fps = 0
			DisplayServer.window_set_vsync_mode(DisplayServer.VSYNC_ENABLED)
			physics_fps = 60  # Cap at 60 for vsync (could be higher)
		ConfigData.FpsLimitMode.NO_LIMIT:
			Engine.max_fps = 0
			DisplayServer.window_set_vsync_mode(DisplayServer.VSYNC_DISABLED)
			physics_fps = 60  # Cap at 60 for unlimited
		ConfigData.FpsLimitMode.FPS_18:
			Engine.max_fps = 18
			DisplayServer.window_set_vsync_mode(DisplayServer.VSYNC_DISABLED)
			physics_fps = 18
		ConfigData.FpsLimitMode.FPS_30:
			Engine.max_fps = 30
			DisplayServer.window_set_vsync_mode(DisplayServer.VSYNC_DISABLED)
			physics_fps = 30
		ConfigData.FpsLimitMode.FPS_60:
			Engine.max_fps = 60
			DisplayServer.window_set_vsync_mode(DisplayServer.VSYNC_DISABLED)
			physics_fps = 60
		ConfigData.FpsLimitMode.FPS_120:
			Engine.max_fps = 120
			DisplayServer.window_set_vsync_mode(DisplayServer.VSYNC_DISABLED)
			physics_fps = 60  # Cap physics at 60

	Engine.physics_ticks_per_second = physics_fps


## Apply a graphic profile by index
## 0: Performance (LOW), 1: Balanced (MEDIUM), 2: Quality (HIGH)
## Does not apply to Custom (3) profiles
static func apply_graphic_profile(profile_index: int) -> void:
	var config := Global.get_config()

	match profile_index:
		0:  # Performance (LOW)
			config.anti_aliasing = 0  # off
			config.shadow_quality = 0  # disabled
			config.skybox = 0  # basic
			config.texture_quality = 0  # low
		1:  # Balanced (MEDIUM)
			config.anti_aliasing = 1  # x2
			config.shadow_quality = 1  # normal
			config.skybox = 1  # normal
			config.texture_quality = 1  # medium
		2:  # Quality (HIGH)
			config.anti_aliasing = 3  # x8
			config.shadow_quality = 2  # high quality
			config.skybox = 2  # realistic
			config.texture_quality = 2  # high
		_:
			# Custom or invalid - do nothing
			return

	config.graphic_profile = profile_index
