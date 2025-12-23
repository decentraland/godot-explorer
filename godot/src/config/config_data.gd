class_name ConfigData
extends DclConfig

signal param_changed(param: ConfigParams)

enum FpsLimitMode {
	VSYNC = 0,
	NO_LIMIT = 1,
	FPS_30 = 2,
	FPS_60 = 3,
	FPS_18 = 4,
}

enum ConfigParams {
	CONTENT_DIRECTORY,
	WINDOW_MODE,
	UI_ZOOM,
	RESOLUTION_3D_SCALE,
	PROCESS_TICK_QUOTA_MS,
	SHOW_FPS,
	LIMIT_FPS,
	SKY_BOX,
	SESSION_ACCOUNT,
	GUEST_PROFILE,
	AUDIO_GENERAL_VOLUME,
	SHADOW_QUALITY,
	BLOOM_QUALITY,
	ANTI_ALIASING,
	GRAPHIC_PROFILE,
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

var process_tick_quota_ms: int = 10:
	set(value):
		process_tick_quota_ms = value
		param_changed.emit(ConfigParams.PROCESS_TICK_QUOTA_MS)

var show_fps: bool = true:
	set(value):
		show_fps = value
		param_changed.emit(ConfigParams.SHOW_FPS)

var dynamic_skybox: bool = true:
	set(value):
		dynamic_skybox = value
		param_changed.emit(ConfigParams.DYNAMIC_SKYBOX)

var skybox_time: int = 43200:
	set(value):
		skybox_time = value
		param_changed.emit(ConfigParams.SKYBOX_TIME)

# See FpsLimitMode enum for available options
var limit_fps: int = FpsLimitMode.FPS_30:
	set(value):
		limit_fps = value
		param_changed.emit(ConfigParams.LIMIT_FPS)

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

# 0: Off, 1: Low, 2: High
var bloom_quality: int = 0:
	set(value):
		bloom_quality = value
		param_changed.emit(ConfigParams.BLOOM_QUALITY)

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

var optimized_assets_version: int = 0

var local_notifications_version: int = 0

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
	self.process_tick_quota_ms = 10
	self.limit_fps = FpsLimitMode.FPS_30

	self.skybox = 0  # basic

	self.shadow_quality = 0  # disabled
	self.bloom_quality = 0  # off
	self.anti_aliasing = 0  # off
	self.graphic_profile = 0

	self.local_content_dir = OS.get_user_data_dir() + "/content"
	self.max_cache_size = 1

	self.show_fps = true

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

	self.process_tick_quota_ms = settings_file.get_value(
		"config", "process_tick_quota_ms", data_default.process_tick_quota_ms
	)

	self.limit_fps = settings_file.get_value("config", "fps_limit", data_default.limit_fps)
	self.skybox = settings_file.get_value("config", "skybox", data_default.skybox)
	self.shadow_quality = settings_file.get_value(
		"config", "shadow_quality", data_default.shadow_quality
	)
	self.bloom_quality = settings_file.get_value(
		"config", "bloom_quality", data_default.bloom_quality
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

	self.optimized_assets_version = settings_file.get_value(
		"user", "optimized_assets_version", data_default.optimized_assets_version
	)

	self.local_notifications_version = settings_file.get_value(
		"user", "local_notifications_version", data_default.local_notifications_version
	)


func save_to_settings_file():
	if Global.testing_scene_mode:
		return

	var new_settings_file: ConfigFile = ConfigFile.new()
	new_settings_file.set_value("config", "process_tick_quota_ms", self.process_tick_quota_ms)
	new_settings_file.set_value("config", "fps_limit", self.limit_fps)
	new_settings_file.set_value("config", "skybox", self.skybox)
	new_settings_file.set_value("config", "shadow_quality", self.shadow_quality)
	new_settings_file.set_value("config", "bloom_quality", self.bloom_quality)
	new_settings_file.set_value("config", "anti_aliasing", self.anti_aliasing)
	new_settings_file.set_value("config", "graphic_profile", self.graphic_profile)
	new_settings_file.set_value("config", "local_content_dir", self.local_content_dir)
	new_settings_file.set_value("config", "max_cache_size", self.max_cache_size)
	new_settings_file.set_value("config", "show_fps", self.show_fps)
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
	new_settings_file.set_value("user", "last_parcel_position", self.last_parcel_position)
	new_settings_file.set_value("user", "last_realm_joined", self.last_realm_joined)
	new_settings_file.set_value("user", "last_places", self.last_places)
	new_settings_file.set_value(
		"user", "terms_and_conditions_version", self.terms_and_conditions_version
	)
	new_settings_file.set_value("user", "optimized_assets_version", self.optimized_assets_version)
	new_settings_file.set_value(
		"user", "local_notifications_version", self.local_notifications_version
	)
	new_settings_file.set_value("analytics", "user_id", self.analytics_user_id)
	new_settings_file.save(DclConfig.get_settings_file_path())
