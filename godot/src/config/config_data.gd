class_name ConfigData
extends RefCounted

signal param_changed(param: ConfigParams, new_value)

enum ConfigParams {
	CONTENT_DIRECTORY,
	WINDOWED,
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
	AUDIO_GENERAL_VOLUME
}

const SETTINGS_FILE = "user://settings.cfg"

var local_content_dir: String = OS.get_user_data_dir() + "/content":
	set(value):
		if DirAccess.dir_exists_absolute(value):
			local_content_dir = value
			param_changed.emit(ConfigParams.CONTENT_DIRECTORY)

var gravity: float = 55.0:
	set(value):
		gravity = value
		param_changed.emit(ConfigParams.GRAVITY)

var windowed: bool = true:
	set(value):
		windowed = value
		param_changed.emit(ConfigParams.WINDOWED)

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

# 0 - Vsync, 1 - No limit, Other-> Limit limit_fps that amount
var limit_fps: int = 0:
	set(value):
		limit_fps = value
		param_changed.emit(ConfigParams.GRAVITY)

# 0- without, 1 - pretty, skybox -default env
var skybox: int = 1:
	set(value):
		skybox = value
		param_changed.emit(ConfigParams.SKY_BOX)

var last_realm_joined: String = "":
	set(value):
		last_realm_joined = value

var last_parcel_position: Vector2i = Vector2i(72, -10):
	set(value):
		last_parcel_position = value

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

func fix_last_places_duplicates(place_dict: Dictionary, _last_places: Array):
	var realm = place_dict.get("realm")
	var position = place_dict.get("position")
	var to_remove: Array[int] = []
	for i in range(_last_places.size()):
		var place = _last_places[i]
		var place_realm = place.get("realm")
		var place_position = place.get("position")
		if place.get("realm") == realm:
			if Realm.is_genesis_city(realm) and place_position == position:
				to_remove.push_front(i)
			else:
				to_remove.push_front(i)
				
	for i in to_remove:
		_last_places.remove_at(i)


func add_place_to_last_places(position: Vector2i, realm: String):
	prints("add_place_to_last_places", realm, position)
	var place_dict = {
		"position": position,
		"realm": realm,
	}
	fix_last_places_duplicates(place_dict, last_places)

	last_places.push_front(place_dict)

	if last_places.size() >= 10:
		last_places.pop_back()


var audio_general_volume: float = 100.0:
	set(value):
		audio_general_volume = value
		param_changed.emit(ConfigParams.AUDIO_GENERAL_VOLUME)


func load_from_default():
	self.gravity = 55.0
	self.jump_velocity = 12.0
	self.walk_velocity = 2.0
	self.run_velocity = 6.0

	self.process_tick_quota_ms = 10
	self.scene_radius = 2
	self.limit_fps = 0

	if Global.is_mobile():
		self.skybox = 0
	else:
		self.skybox = 1

	self.local_content_dir = OS.get_user_data_dir() + "/content"

	self.show_fps = true

	self.windowed = true

	self.session_account = {}
	self.guest_profile = {}

	self.last_realm_joined = "https://sdk-team-cdn.decentraland.org/ipfs/goerli-plaza-main"
	self.last_parcel_position = Vector2i(72, -10)


func load_from_settings_file():
	var data_default := ConfigData.new()
	data_default.load_from_default()

	var settings_file: ConfigFile = ConfigFile.new()
	settings_file.load(SETTINGS_FILE)

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
	self.scene_radius = settings_file.get_value("config", "scene_radius", data_default.scene_radius)
	self.limit_fps = settings_file.get_value("config", "limit_fps", data_default.limit_fps)
	self.skybox = settings_file.get_value("config", "skybox", data_default.skybox)
	self.local_content_dir = settings_file.get_value(
		"config", "local_content_dir", data_default.local_content_dir
	)
	self.show_fps = settings_file.get_value("config", "show_fps", data_default.show_fps)
	self.windowed = settings_file.get_value("config", "windowed", data_default.windowed)
	self.ui_zoom = settings_file.get_value("config", "ui_zoom", data_default.ui_zoom)
	self.resolution_3d_scale = settings_file.get_value(
		"config", "resolution_3d_scale", data_default.resolution_3d_scale
	)
	self.audio_general_volume = settings_file.get_value(
		"config", "audio_general_volume", data_default.audio_general_volume
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

	self.last_places = settings_file.get_value(
		"user", "last_places", data_default.last_places
	)


func save_to_settings_file():
	if Global.testing_scene_mode:
		return

	var settings_file: ConfigFile = ConfigFile.new()
	settings_file.set_value("config", "gravity", self.gravity)
	settings_file.set_value("config", "jump_velocity", self.jump_velocity)
	settings_file.set_value("config", "walk_velocity", self.walk_velocity)
	settings_file.set_value("config", "run_velocity", self.run_velocity)
	settings_file.set_value("config", "process_tick_quota_ms", self.process_tick_quota_ms)
	settings_file.set_value("config", "scene_radius", self.scene_radius)
	settings_file.set_value("config", "limit_fps", self.limit_fps)
	settings_file.set_value("config", "skybox", self.skybox)
	settings_file.set_value("config", "local_content_dir", self.local_content_dir)
	settings_file.set_value("config", "show_fps", self.show_fps)
	settings_file.set_value("config", "windowed", self.windowed)
	settings_file.set_value("config", "ui_zoom", self.ui_zoom)
	settings_file.set_value("config", "resolution_3d_scale", self.resolution_3d_scale)
	settings_file.set_value("config", "audio_general_volume", self.audio_general_volume)
	settings_file.set_value("session", "account", self.session_account)
	settings_file.set_value("session", "guest_profile", self.guest_profile)
	settings_file.set_value("user", "last_parcel_position", self.last_parcel_position)
	settings_file.set_value("user", "last_realm_joined", self.last_realm_joined)
	settings_file.set_value("user", "last_places", self.last_places)
	settings_file.save(SETTINGS_FILE)
