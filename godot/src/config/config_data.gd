class_name ConfigData
extends DclConfig

signal param_changed(param: ConfigParams)

enum ConfigParams {
	CONTENT_DIRECTORY,
	WINDOW_MODE,
	UI_ZOOM,
	RESOLUTION_3D_SCALE,
	GRAVITY,
	JUMP_VELOCITY,
	WALK_VELOCITY,
	RUN_VELOCITY,
	PROCESS_TICK_QUOTA_MS,
	SCENE_RADIUS,
	SHOW_FPS,
	LIMIT_FPS,
	SKY_BOX,
	SESSION_ACCOUNT,
	GUEST_PROFILE,
	AUDIO_GENERAL_VOLUME,
	SHADOW_QUALITY,
	ANTI_ALIASING,
	GRAPHIC_PROFILE,
	LOADING_SCENES_ARROUND,
	DYNAMIC_SKYBOX,
	SKYBOX_TIME,
}

var local_content_dir: String = OS.get_user_data_dir() + "/content":
	set(value):
		if DirAccess.dir_exists_absolute(value):
			local_content_dir = value
			param_changed.emit(ConfigParams.CONTENT_DIRECTORY)

# 0=512mb 1=1gb 2=2gb
var max_cache_size: int = 1:
	set(value):
		max_cache_size = value

var gravity: float = 55.0:
	set(value):
		gravity = value
		param_changed.emit(ConfigParams.GRAVITY)

# 0: Windowed, 1: Borderless, 2: Full Screen
var window_mode: int = 0:
	set(value):
		window_mode = value
		param_changed.emit(ConfigParams.WINDOW_MODE)

var ui_zoom: float = -1.0:
	set(value):
		ui_zoom = value
		param_changed.emit(ConfigParams.UI_ZOOM)

var resolution_3d_scale: float = 1.0:
	set(value):
		resolution_3d_scale = value
		param_changed.emit(ConfigParams.RESOLUTION_3D_SCALE)

var jump_velocity: float = 12.0:
	set(value):
		jump_velocity = value
		param_changed.emit(ConfigParams.JUMP_VELOCITY)

var walk_velocity: float = 2.0:
	set(value):
		walk_velocity = value
		param_changed.emit(ConfigParams.WALK_VELOCITY)

var run_velocity: float = 6.0:
	set(value):
		run_velocity = value
		param_changed.emit(ConfigParams.RUN_VELOCITY)

var process_tick_quota_ms: int = 10:
	set(value):
		process_tick_quota_ms = value
		param_changed.emit(ConfigParams.PROCESS_TICK_QUOTA_MS)

var scene_radius: int = 2:
	set(value):
		scene_radius = value
		param_changed.emit(ConfigParams.SCENE_RADIUS)

var show_fps: bool = true:
	set(value):
		show_fps = value
		param_changed.emit(ConfigParams.SHOW_FPS)

var loading_scene_arround_only_when_you_pass: bool = true:
	set(value):
		loading_scene_arround_only_when_you_pass = value
		param_changed.emit(ConfigParams.LOADING_SCENES_ARROUND)

var dynamic_skybox: bool = true:
	set(value):
		dynamic_skybox = value
		param_changed.emit(ConfigParams.DYNAMIC_SKYBOX)

var skybox_time: int = 43200:
	set(value):
		skybox_time = value
		param_changed.emit(ConfigParams.SKYBOX_TIME)

# 0 - Vsync, 1 - No limit, Other-> Limit limit_fps that amount
var limit_fps: int = 0:
	set(value):
		limit_fps = value
		param_changed.emit(ConfigParams.GRAVITY)

# 0- performance, 1- balanced, 2- high quality
var skybox: int = 0:
	set(value):
		skybox = value
		param_changed.emit(ConfigParams.SKY_BOX)

# 0- no shadow, 1- low res shadow, 2- high res shadow
var shadow_quality: int = 0:
	set(value):
		shadow_quality = value
		param_changed.emit(ConfigParams.SHADOW_QUALITY)

# 0: Performance, 1: Balanced, 2: Quality, 3: Custom
var graphic_profile: int = 0:
	set(value):
		graphic_profile = value
		param_changed.emit(ConfigParams.GRAPHIC_PROFILE)

# 0: Off, 1: x2, 2: x4, 3: x8
var anti_aliasing: int = 0:
	set(value):
		anti_aliasing = value
		param_changed.emit(ConfigParams.ANTI_ALIASING)

var last_realm_joined: String = "":
	set(value):
		last_realm_joined = value

var last_parcel_position: Vector2i = Vector2i(72, -10):
	set(value):
		last_parcel_position = value

var terms_and_conditions_version: int = 0

var last_places: Array[Dictionary] = []:
	set(value):
		last_places = value

var session_account: Dictionary = {}:
	set(value):
		session_account = value
		param_changed.emit(ConfigParams.SESSION_ACCOUNT)

var guest_profile: Dictionary = {}:
	set(value):
		guest_profile = value
		param_changed.emit(ConfigParams.GUEST_PROFILE)

var temporary_blocked_list: Array = []:
	set(value):
		temporary_blocked_list = value

var temporary_muted_list: Array = []:
	set(value):
		temporary_muted_list = value

var audio_general_volume: float = 100.0:
	set(value):
		audio_general_volume = value
		param_changed.emit(ConfigParams.AUDIO_GENERAL_VOLUME)

var audio_scene_volume: float = 100.0:
	set(value):
		audio_scene_volume = value

var audio_voice_chat_volume: float = 100.0:
	set(value):
		audio_voice_chat_volume = value

var audio_ui_volume: float = 100.0:
	set(value):
		audio_ui_volume = value

var audio_music_volume: float = 100.0:
	set(value):
		audio_music_volume = value

var audio_mic_amplification: float = 100.0:
	set(value):
		audio_mic_amplification = value

var analytics_user_id: String = "":
	set(value):
		analytics_user_id = value


func fix_last_places_duplicates(place_dict: Dictionary, _last_places: Array):
	var realm = place_dict.get("realm")
	var position = place_dict.get("position")
	var to_remove: Array = []
	for place in _last_places:
		var place_realm = place.get("realm")
		var place_position = place.get("position")
		if place_realm == realm:
			if Realm.is_genesis_city(realm):
				if place_position == position:
					to_remove.push_back(place)
			else:
				to_remove.push_back(place)

	for place in to_remove:
		_last_places.erase(place)


func add_place_to_last_places(position: Vector2i, realm: String):
	var place_dict = {
		"position": position,
		"realm": realm,
	}
	fix_last_places_duplicates(place_dict, last_places)

	last_places.push_front(place_dict)

	if last_places.size() >= 10:
		last_places.pop_back()


func load_from_default():
	self.gravity = 55.0
	self.jump_velocity = 12.0
	self.walk_velocity = 2.0
	self.run_velocity = 6.0

	self.process_tick_quota_ms = 10
	self.scene_radius = 2
	self.limit_fps = 0

	self.skybox = 0  # basic

	self.shadow_quality = 0  # disabled
	self.anti_aliasing = 0  # off
	self.graphic_profile = 0

	self.local_content_dir = OS.get_user_data_dir() + "/content"
	self.max_cache_size = 1

	self.show_fps = true
	self.loading_scene_arround_only_when_you_pass = true

	self.dynamic_skybox = true
	self.skybox_time = 43200

	self.window_mode = 0

	self.session_account = {}
	self.guest_profile = {}

	self.last_realm_joined = "https://sdk-team-cdn.decentraland.org/ipfs/goerli-plaza-main-latest"
	self.last_parcel_position = Vector2i(72, -10)

	self.analytics_user_id = DclConfig.generate_uuid_v4()


func load_from_settings_file():
	var data_default := ConfigData.new()
	data_default.load_from_default()

	self.gravity = settings_file.get_value("config", "gravity", data_default.gravity)
	self.jump_velocity = settings_file.get_value(
		"config", "jump_velocity", data_default.jump_velocity
	)
	self.walk_velocity = settings_file.get_value(
		"config", "walk_velocity", data_default.walk_velocity
	)
	self.run_velocity = settings_file.get_value("config", "run_velocity", data_default.run_velocity)
	self.process_tick_quota_ms = settings_file.get_value(
		"config", "process_tick_quota_ms", data_default.process_tick_quota_ms
	)

	# TODO: Change the way of loading the scenes in XR (https://github.com/decentraland/godot-explorer/issues/274)
	if Global.is_xr():
		self.scene_radius = 1
	else:
		self.scene_radius = settings_file.get_value(
			"config", "scene_radius", data_default.scene_radius
		)
	self.limit_fps = settings_file.get_value("config", "limit_fps", data_default.limit_fps)
	self.skybox = settings_file.get_value("config", "skybox", data_default.skybox)
	self.shadow_quality = settings_file.get_value(
		"config", "shadow_quality", data_default.shadow_quality
	)
	self.anti_aliasing = settings_file.get_value(
		"config", "anti_aliasing", data_default.anti_aliasing
	)
	self.graphic_profile = settings_file.get_value(
		"config", "graphic_profile", data_default.graphic_profile
	)
	self.local_content_dir = settings_file.get_value(
		"config", "local_content_dir", data_default.local_content_dir
	)

	self.max_cache_size = settings_file.get_value(
		"config", "max_cache_size", data_default.max_cache_size
	)
	self.show_fps = settings_file.get_value("config", "show_fps", data_default.show_fps)
	self.loading_scene_arround_only_when_you_pass = settings_file.get_value(
		"config",
		"loading_scene_arround_only_when_you_pass",
		data_default.loading_scene_arround_only_when_you_pass
	)

	self.dynamic_skybox = settings_file.get_value(
		"config", "dynamic_skybox", data_default.dynamic_skybox
	)
	self.skybox_time = settings_file.get_value("config", "skybox_time", data_default.skybox_time)

	self.window_mode = settings_file.get_value("config", "window_mode", data_default.window_mode)
	self.ui_zoom = settings_file.get_value("config", "ui_zoom", data_default.ui_zoom)
	self.resolution_3d_scale = settings_file.get_value(
		"config", "resolution_3d_scale", data_default.resolution_3d_scale
	)

	self.audio_general_volume = settings_file.get_value(
		"config", "audio_general_volume", data_default.audio_general_volume
	)

	self.audio_scene_volume = settings_file.get_value(
		"config", "audio_scene_volume", data_default.audio_scene_volume
	)

	self.audio_voice_chat_volume = settings_file.get_value(
		"config", "audio_voice_chat_volume", data_default.audio_voice_chat_volume
	)

	self.audio_ui_volume = settings_file.get_value(
		"config", "audio_ui_volume", data_default.audio_ui_volume
	)

	self.audio_music_volume = settings_file.get_value(
		"config", "audio_music_volume", data_default.audio_music_volume
	)

	self.audio_mic_amplification = settings_file.get_value(
		"config", "audio_mic_amplification", data_default.audio_mic_amplification
	)

	self.session_account = settings_file.get_value(
		"session", "account", data_default.session_account
	)

	self.guest_profile = settings_file.get_value(
		"session", "guest_profile", data_default.guest_profile
	)

	self.temporary_blocked_list = settings_file.get_value(
		"session", "temporary_blocked_list", data_default.temporary_blocked_list
	)

	self.temporary_muted_list = settings_file.get_value(
		"session", "temporary_muted_list", data_default.temporary_muted_list
	)

	self.last_parcel_position = settings_file.get_value(
		"user", "last_parcel_position", data_default.last_parcel_position
	)

	self.last_realm_joined = settings_file.get_value(
		"user", "last_realm_joined", data_default.last_realm_joined
	)

	self.analytics_user_id = settings_file.get_value(
		"analytics", "user_id", DclConfig.generate_uuid_v4()
	)

	self.last_places = settings_file.get_value("user", "last_places", data_default.last_places)

	self.terms_and_conditions_version = settings_file.get_value(
		"user", "terms_and_conditions_version", data_default.terms_and_conditions_version
	)


func save_to_settings_file():
	if Global.testing_scene_mode:
		return

	var new_settings_file: ConfigFile = ConfigFile.new()
	new_settings_file.set_value("config", "gravity", self.gravity)
	new_settings_file.set_value("config", "jump_velocity", self.jump_velocity)
	new_settings_file.set_value("config", "walk_velocity", self.walk_velocity)
	new_settings_file.set_value("config", "run_velocity", self.run_velocity)
	new_settings_file.set_value("config", "process_tick_quota_ms", self.process_tick_quota_ms)
	new_settings_file.set_value("config", "scene_radius", self.scene_radius)
	new_settings_file.set_value("config", "limit_fps", self.limit_fps)
	new_settings_file.set_value("config", "skybox", self.skybox)
	new_settings_file.set_value("config", "shadow_quality", self.shadow_quality)
	new_settings_file.set_value("config", "anti_aliasing", self.anti_aliasing)
	new_settings_file.set_value("config", "graphic_profile", self.graphic_profile)
	new_settings_file.set_value("config", "local_content_dir", self.local_content_dir)
	new_settings_file.set_value("config", "max_cache_size", self.max_cache_size)
	new_settings_file.set_value("config", "show_fps", self.show_fps)
	new_settings_file.set_value(
		"config",
		"loading_scene_arround_only_when_you_pass",
		self.loading_scene_arround_only_when_you_pass
	)
	new_settings_file.set_value("config", "dynamic_skybox", self.dynamic_skybox)
	new_settings_file.set_value("config", "skybox_time", self.skybox_time)
	new_settings_file.set_value("config", "window_mode", self.window_mode)
	new_settings_file.set_value("config", "ui_zoom", self.ui_zoom)
	new_settings_file.set_value("config", "resolution_3d_scale", self.resolution_3d_scale)
	new_settings_file.set_value("config", "audio_general_volume", self.audio_general_volume)
	new_settings_file.set_value("config", "audio_scene_volume", self.audio_scene_volume)
	new_settings_file.set_value("config", "audio_ui_volume", self.audio_ui_volume)
	new_settings_file.set_value("config", "audio_music_volume", self.audio_music_volume)
	new_settings_file.set_value("config", "audio_voice_chat_volume", self.audio_voice_chat_volume)
	new_settings_file.set_value("config", "audio_mic_amplification", self.audio_mic_amplification)
	new_settings_file.set_value("config", "texture_quality", self.get_texture_quality())
	new_settings_file.set_value("session", "account", self.session_account)
	new_settings_file.set_value("session", "guest_profile", self.guest_profile)
	new_settings_file.set_value("session", "temporary_blocked_list", self.temporary_blocked_list)
	new_settings_file.set_value("session", "temporary_muted_list", self.temporary_muted_list)
	new_settings_file.set_value("user", "last_parcel_position", self.last_parcel_position)
	new_settings_file.set_value("user", "last_realm_joined", self.last_realm_joined)
	new_settings_file.set_value("user", "last_places", self.last_places)
	new_settings_file.set_value(
		"user", "terms_and_conditions_version", self.terms_and_conditions_version
	)
	new_settings_file.set_value("analytics", "user_id", self.analytics_user_id)
	new_settings_file.save(DclConfig.get_settings_file_path())
