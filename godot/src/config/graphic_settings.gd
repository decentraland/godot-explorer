class_name GraphicSettings extends RefCounted

## Profile definitions as data - easier to tune without code changes
## Keys: aa, shadow, bloom, skybox, texture, fps, scale, mesh_lod_threshold, view_distance, particle_quality
##
## mesh_lod_threshold (pixels of screen-space error before swapping to a
## lower-detail LOD). Default Godot=1.0 — very conservative. Genesis Plaza
## scenes ship a full LOD chain via the Rust import pipeline; raising the
## threshold makes that chain do real work by swapping farther/smaller MIs
## to LOD1/2/3 sooner. Tuned per profile so HIGH stays visually crisp
## while lower profiles trade detail for fragment cost.
##
## dcl_lights / dcl_light_shadows / dcl_max_lights / dcl_light_range_cap:
## scene-authored dynamic lights. Mobile renderer allows 8 lights per mesh,
## and each active light adds per-fragment cost — so lower profiles cap the
## global active-light budget hard and disable dynamic light shadows.
## range_cap clamps the intensity-derived auto activation range so very
## bright authored lights can't activate from across the scene.
const PROFILE_DEFINITIONS: Array[Dictionary] = [
	# Very Low (0) - Maximum battery savings
	{
		"aa": 0,
		"shadow": 0,
		"bloom": 0,
		"skybox": 0,
		"texture": 0,
		"fps": ConfigData.FpsLimitMode.FPS_18,
		"scale": 0.5,
		"mesh_lod_threshold": 8.0,
		"view_distance": 40.0,
		"particle_quality": 0,
		"dcl_lights": false,
		"dcl_light_shadows": false,
		"dcl_max_lights": 0,
		"dcl_light_range_cap": 15.0
	},
	# Low (1) - Battery savings with better visuals
	{
		"aa": 0,
		"shadow": 0,
		"bloom": 0,
		"skybox": 0,
		"texture": 0,
		"fps": ConfigData.FpsLimitMode.FPS_30,
		"scale": 0.75,
		"mesh_lod_threshold": 6.0,
		"view_distance": 60.0,
		"particle_quality": 1,
		"dcl_lights": true,
		"dcl_light_shadows": false,
		"dcl_max_lights": 2,
		"dcl_light_range_cap": 20.0
	},
	# Medium (2) - Balanced performance and quality. AA off (MSAA_DISABLED)
	# because A/B on Mali-G68 showed it costs ~8ms gpu in Genesis Plaza
	# without a meaningful look-and-feel change at typical mobile resolution.
	{
		"aa": 0,
		"shadow": 1,
		"bloom": 1,
		"skybox": 1,
		"texture": 1,
		"fps": ConfigData.FpsLimitMode.FPS_30,
		"scale": 1.0,
		"mesh_lod_threshold": 3.0,
		"view_distance": 80.0,
		"particle_quality": 2,
		"dcl_lights": true,
		"dcl_light_shadows": false,
		"dcl_max_lights": 4,
		"dcl_light_range_cap": 30.0
	},
	# High (3) - Best quality. shadow capped at 1 (Low) because Genesis
	# Plaza bench on Mali-G68 showed shadow_quality=2 costs ~10ms gpu
	# vs Low without buying meaningful visual quality at typical mobile
	# resolution (62% of the geometry goes through the shadow pass; Low
	# keeps shadows visually present with -22% prim and -36% draws,
	# recovering ~5 fps over High).
	{
		"aa": 1,
		"shadow": 1,
		"bloom": 2,
		"skybox": 2,
		"texture": 2,
		"fps": ConfigData.FpsLimitMode.FPS_60,
		"scale": 1.0,
		"mesh_lod_threshold": 2.0,
		"view_distance": 300.0,
		"particle_quality": 3,
		"dcl_lights": true,
		"dcl_light_shadows": true,
		"dcl_max_lights": 8,
		"dcl_light_range_cap": 40.0
	},
]

## English, and deliberately so: these feed print() logs (global.gd, settings.gd) and the debug
## FPS overlay (explorer.gd), which stay English. The user-facing dropdown uses PROFILE_KEYS.
# i18n-keys: SETTINGS_GRAPHIC_PROFILE_*, SETTINGS_SKYBOX_*
const PROFILE_NAMES: Array[String] = ["Very Low", "Low", "Medium", "High", "Custom"]

## Display keys for PROFILE_NAMES, same order. A parallel table rather than a rename, so the
## log/overlay call sites keep their English names.
const PROFILE_KEYS: Array[String] = [
	"SETTINGS_GRAPHIC_PROFILE_VERY_LOW",
	"SETTINGS_GRAPHIC_PROFILE_LOW",
	"SETTINGS_GRAPHIC_PROFILE_MEDIUM",
	"SETTINGS_GRAPHIC_PROFILE_HIGH",
	"SETTINGS_GRAPHIC_PROFILE_CUSTOM",
]
## `secs` is the value that is stored and matched; `name` is English for logs and `key` is what
## the Settings dropdown shows.
const SKYBOX_TIME_NAMES: Array[Dictionary] = [
	{"name": "Midnight", "key": "SETTINGS_SKYBOX_MIDNIGHT", "secs": 86400},
	{"name": "Afternoon", "key": "SETTINGS_SKYBOX_AFTERNOON", "secs": 64800},
	{"name": "Midday", "key": "SETTINGS_SKYBOX_MIDDAY", "secs": 43200},
	{"name": "Morning", "key": "SETTINGS_SKYBOX_MORNING", "secs": 21600}
]


static func connect_global_signal(root: Window):
	root.size_changed.connect(GraphicSettings.apply_ui_zoom.bind(root))
	# Orientation flips swap the base resolution axes, so the scale must be
	# recomputed. Defer to let the window finish resizing (mobile rotates async).
	Global.orientation_changed.connect(
		func(_is_portrait): GraphicSettings.apply_ui_zoom.bind(root).call_deferred()
	)


## Returns the design base resolution from project settings, with axes swapped
## when the current window is portrait so that the design space follows the
## screen orientation (e.g. 1600x720 landscape -> 720x1600 portrait).
static func get_base_resolution(screen_size: Vector2) -> Vector2:
	var base_width: float = ProjectSettings.get_setting("display/window/size/viewport_width", 1600)
	var base_height: float = ProjectSettings.get_setting("display/window/size/viewport_height", 720)
	var base_is_portrait: bool = base_height > base_width
	var screen_is_portrait: bool = screen_size.y > screen_size.x
	if base_is_portrait != screen_is_portrait:
		return Vector2(base_height, base_width)
	return Vector2(base_width, base_height)


static func get_max_ui_zoom(root: Window) -> float:
	var screen_size: Vector2 = root.size
	var base_resolution: Vector2 = get_base_resolution(screen_size)

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
	if screen_size.x <= 0 or screen_size.y <= 0:
		return

	# On real mobile devices the OS can transiently report swapped dimensions
	# (portrait height as width and vice-versa) during iOS background/foreground
	# transitions. Applying the scale at that moment would produce an incorrect
	# content_scale_factor that makes the UI lay out in landscape while the screen
	# is actually portrait (card wider than display, etc.). Guard against this by
	# skipping any resize event whose orientation contradicts the authoritative
	# orientation tracked by Global.
	if Global.is_mobile() and not Global.is_virtual_mobile():
		var size_says_portrait: bool = screen_size.y > screen_size.x
		if size_says_portrait != Global.is_orientation_portrait():
			return

	var base_resolution: Vector2 = get_base_resolution(screen_size)
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


## Convert FpsLimitMode enum to actual FPS value (0 = unlimited)
static func fps_limit_mode_to_fps(mode: int) -> int:
	match mode:
		ConfigData.FpsLimitMode.VSYNC:
			return 0
		ConfigData.FpsLimitMode.NO_LIMIT:
			return 0
		ConfigData.FpsLimitMode.FPS_18:
			return 18
		ConfigData.FpsLimitMode.FPS_30:
			return 30
		ConfigData.FpsLimitMode.FPS_60:
			return 60
		_:
			return 30


static func apply_fps_limit():
	apply_fps_limit_with_thermal_cap(Global.get_config().limit_fps, 0)


## Apply FPS limit considering both user setting and thermal cap
static func apply_fps_limit_with_thermal_cap(fps_limit_mode: int, thermal_fps_cap: int):
	var user_fps := fps_limit_mode_to_fps(fps_limit_mode)

	# Determine effective FPS (use more restrictive)
	var effective_fps: int = user_fps
	if thermal_fps_cap > 0:
		if user_fps == 0:
			effective_fps = thermal_fps_cap
		else:
			effective_fps = mini(user_fps, thermal_fps_cap)

	var physics_fps := 30

	# Handle VSYNC specially
	if fps_limit_mode == ConfigData.FpsLimitMode.VSYNC and thermal_fps_cap == 0:
		Engine.max_fps = 0
		DisplayServer.window_set_vsync_mode(DisplayServer.VSYNC_ENABLED)
		physics_fps = 60
	elif effective_fps == 0:
		Engine.max_fps = 0
		DisplayServer.window_set_vsync_mode(DisplayServer.VSYNC_DISABLED)
		physics_fps = 60
	else:
		Engine.max_fps = effective_fps
		DisplayServer.window_set_vsync_mode(DisplayServer.VSYNC_DISABLED)
		physics_fps = mini(effective_fps, 60)

	Engine.physics_ticks_per_second = physics_fps


static func apply_low_processor_mode() -> void:
	# For lobby/menus - reduce CPU usage
	OS.low_processor_usage_mode = true
	DisplayServer.window_set_vsync_mode(DisplayServer.VSYNC_ENABLED)
	Engine.max_fps = 0  # Let VSync control frame rate
	Engine.physics_ticks_per_second = 60
	Global.get_window().get_viewport().disable_3d = true


static func apply_full_processor_mode() -> void:
	# For world exploration - full performance
	OS.low_processor_usage_mode = false
	apply_fps_limit()  # Apply user's configured FPS limit
	Global.get_window().get_viewport().disable_3d = false


## Apply a graphic profile by index
## 0: Very Low, 1: Low, 2: Medium, 3: High, 4: Custom
## Sets ALL graphics parameters including FPS limit, bloom, and 3D resolution scale
static func apply_graphic_profile(profile_index: int) -> void:
	# Custom or invalid index - do nothing
	if profile_index < 0 or profile_index >= PROFILE_DEFINITIONS.size():
		return

	var config: DclConfig = Global.get_config()
	var profile: Dictionary = PROFILE_DEFINITIONS[profile_index]

	# Apply all settings from profile definition
	config.anti_aliasing = profile.aa
	config.shadow_quality = profile.shadow
	config.bloom_quality = profile.bloom
	config.skybox = profile.skybox
	config.texture_quality = profile.texture
	config.limit_fps = profile.fps
	config.resolution_3d_scale = profile.scale
	config.view_distance = profile.view_distance
	config.particle_quality = profile.particle_quality
	config.graphic_profile = profile_index

	# Apply FPS limit immediately
	apply_fps_limit()

	# Apply view distance immediately
	if is_instance_valid(Global.player_camera_node):
		Global.player_camera_node.far = profile.view_distance

	# Apply 3D resolution scale to viewport
	var viewport := Global.get_tree().root.get_viewport()
	if viewport:
		viewport.scaling_3d_scale = config.resolution_3d_scale
		# Apply per-profile mesh-LOD pixel threshold. Higher value = swap
		# to lower-detail LOD sooner (more aggressive). LOD chain is
		# baked at GLTF import time; this is the only knob that decides
		# how aggressively the renderer picks LOD1/2/3 at distance.
		var lod_thr: float = profile.get("mesh_lod_threshold", 1.0)
		viewport.mesh_lod_threshold = lod_thr

	# Avatar move/jump/land dust particles follow the profile.
	AvatarAnimHelpers.apply_particles_enabled(profile.particle_quality > 0)

	# Scene dynamic lights follow the profile. Dev Tools can override at runtime.
	DclLightSourceComponent.apply_graphic_profile_settings(
		profile.get("dcl_lights", false),
		profile.get("dcl_light_shadows", false),
		profile.get("dcl_max_lights", 0),
		profile.get("dcl_light_range_cap", 15.0)
	)
