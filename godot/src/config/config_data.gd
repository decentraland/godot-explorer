class_name ConfigData extends RefCounted

enum ConfigParams {
	ContentDirectory,
	Resolution,
	WindowSize,
	UiScale,
	Gravity,
	JumpVelocity,
	WalkVelocity,
	RunVelocity,
	ProcessTickQuotaMs,
	SceneRadius,
	ShowFps,
	LimitFps,
	SkyBox,
	AvatarProfile
}

signal param_changed(param: ConfigParams, new_value)

var local_content_dir: String = OS.get_user_data_dir() + "/content":
	set(value):
		if DirAccess.dir_exists_absolute(value):
			local_content_dir = value
			param_changed.emit(ConfigParams.ContentDirectory)

var gravity: float = 55.0:
	set(value):
		gravity = value
		param_changed.emit(ConfigParams.Gravity)

var resolution: String = "1280 x 720":
	set(value):
		resolution = value
		param_changed.emit(ConfigParams.Resolution)

var window_size: String = "1280 x 720":
	set(value):
		window_size = value
		param_changed.emit(ConfigParams.WindowSize)

var ui_scale: float:
	set(value):
		ui_scale = value
		param_changed.emit(ConfigParams.UiScale)

var jump_velocity: float = 12.0:
	set(value):
		jump_velocity = value
		param_changed.emit(ConfigParams.JumpVelocity)

var walk_velocity: float = 2.0:
	set(value):
		walk_velocity = value
		param_changed.emit(ConfigParams.WalkVelocity)

var run_velocity: float = 6.0:
	set(value):
		run_velocity = value
		param_changed.emit(ConfigParams.RunVelocity)

var process_tick_quota_ms: int = 10:
	set(value):
		process_tick_quota_ms = value
		param_changed.emit(ConfigParams.ProcessTickQuotaMs)

var scene_radius: int = 4:
	set(value):
		scene_radius = value
		param_changed.emit(ConfigParams.SceneRadius)

var show_fps: bool = true:
	set(value):
		show_fps = value
		param_changed.emit(ConfigParams.ShowFps)

# 0 - Vsync, 1 - No limit, Other-> Limit limit_fps that amount
var limit_fps: int = 0:
	set(value):
		limit_fps = value
		param_changed.emit(ConfigParams.Gravity)

# 0- without, 1 - pretty, skybox -default env
var skybox: int = 1:
	set(value):
		skybox = value
		param_changed.emit(ConfigParams.SkyBox)

var avatar_profile: Dictionary = {}:
	set(value):
		avatar_profile = value
		param_changed.emit(ConfigParams.AvatarProfile)

var last_realm_joined: String = "":
	set(value):
		last_realm_joined = value

var last_parcel_position: Vector2i = Vector2i(72, -10):
	set(value):
		last_parcel_position = value


func load_from_default():
	self.gravity = 55.0
	self.jump_velocity = 12.0
	self.walk_velocity = 2.0
	self.run_velocity = 6.0

	self.process_tick_quota_ms = 10
	self.scene_radius = 4
	self.limit_fps = 0

	if OS.has_feature("mobile"):
		self.skybox = 0
	else:
		self.skybox = 1

	self.local_content_dir = OS.get_user_data_dir() + "/content"

	self.show_fps = true

	self.resolution = "1280 x 720"
	self.window_size = "1280 x 720"
	self.ui_scale = 1
	self.avatar_profile = {
		"base_url": "https://peer.decentraland.org/content",
		"name": "Godotte",
		"body_shape": "urn:decentraland:off-chain:base-avatars:BaseFemale",
		"eyes": Color(0.3, 0.22, 0.99),
		"hair": Color(0.6, 0.38, 0.1),
		"skin": Color(0.5, 0.36, 0.28),
		"wearables":
		[
			"urn:decentraland:off-chain:base-avatars:f_sweater",
			"urn:decentraland:off-chain:base-avatars:f_jeans",
			"urn:decentraland:off-chain:base-avatars:bun_shoes",
			"urn:decentraland:off-chain:base-avatars:standard_hair",
			"urn:decentraland:off-chain:base-avatars:f_eyes_01",
			"urn:decentraland:off-chain:base-avatars:f_eyebrows_00",
			"urn:decentraland:off-chain:base-avatars:f_mouth_00"
		],
		"emotes": []
	}

	self.last_realm_joined = "https://sdk-team-cdn.decentraland.org/ipfs/goerli-plaza-main"
	self.last_parcel_position = Vector2i(72, -10)


const SETTINGS_FILE = "user://settings.cfg"


func load_from_settings_file():
	var data_default = ConfigData.new()
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
	self.resolution = settings_file.get_value("config", "resolution", data_default.resolution)
	self.window_size = settings_file.get_value("config", "window_size", data_default.window_size)
	self.ui_scale = settings_file.get_value("config", "ui_scale", data_default.ui_scale)

	self.avatar_profile = settings_file.get_value("profile", "avatar", data_default.avatar_profile)
	self.last_parcel_position = settings_file.get_value(
		"user", "last_parcel_position", data_default.last_parcel_position
	)
	self.last_realm_joined = settings_file.get_value(
		"user", "last_realm_joined", data_default.last_realm_joined
	)


func save_to_settings_file():
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
	settings_file.set_value("config", "resolution", self.resolution)
	settings_file.set_value("config", "window_size", self.window_size)
	settings_file.set_value("config", "ui_scale", self.ui_scale)
	settings_file.set_value("profile", "avatar", self.avatar_profile)
	settings_file.set_value("user", "last_parcel_position", self.last_parcel_position)
	settings_file.set_value("user", "last_realm_joined", self.last_realm_joined)
	settings_file.save(SETTINGS_FILE)
