class_name RnReqSettings
extends RefCounted
## settings.* requests: a whitelist of RN-visible keys mapped to ConfigData properties and the same
## apply functions the legacy settings page calls.

const BYTES_PER_MB := 1000 * 1000
const CACHE_SIZE_MB: Array[int] = [1024, 2048, 4096]

# key -> [ConfigData property, kind]. Keys the legacy page does not expose to players are absent.
const KEYS := {
	"submit_message_closes_chat": ["submit_message_closes_chat", "bool"],
	"locale": ["locale", "string"],
	"graphic_profile": ["graphic_profile", "int"],
	"dynamic_graphics_enabled": ["dynamic_graphics_enabled", "bool"],
	"dynamic_skybox": ["dynamic_skybox", "bool"],
	"skybox_time": ["skybox_time", "int"],
	"view_distance": ["view_distance", "float"],
	"particle_quality": ["particle_quality", "int"],
	"limit_fps": ["limit_fps", "int"],
	"max_cache_size": ["max_cache_size", "int"],
	"audio_general_volume": ["audio_general_volume", "float"],
	"audio_scene_volume": ["audio_scene_volume", "float"],
	"audio_voice_chat_volume": ["audio_voice_chat_volume", "float"],
	"audio_ui_volume": ["audio_ui_volume", "float"],
	"audio_music_volume": ["audio_music_volume", "float"],
	"audio_avatar_and_emotes_volume": ["audio_avatar_and_emotes_volume", "float"],
	"audio_mic_amplification": ["audio_mic_amplification", "float"],
}

# Setting a profile rewrites these too, so they are re-announced afterwards.
const PROFILE_DEPENDENT := ["view_distance", "particle_quality", "limit_fps"]

var _emit: Callable


func _init(emit: Callable) -> void:
	_emit = emit


func connect_signals() -> void:
	var manager: Object = Global.dynamic_graphics_manager
	if manager != null and manager.has_signal("profile_change_requested"):
		manager.profile_change_requested.connect(
			func(_profile: int) -> void: _announce("graphic_profile")
		)


func async_handle(method: String, params: Dictionary) -> Dictionary:
	match method:
		"settings.get_all":
			return RnReqUtil.ok(_get_all())
		"settings.set":
			return _apply_setting(params)
		"settings.cache_size":
			return RnReqUtil.ok(_cache_size())
		"settings.clear_cache":
			return await _async_clear_cache()
		"settings.version":
			return RnReqUtil.ok({"version": _display_version()})
	return RnReqUtil.fail("unknown method " + method)


func _get_all() -> Dictionary:
	var config := Global.get_config()
	var out := {}
	for key: String in KEYS:
		out[key] = _read(config, key)
	return out


func _read(config: Object, key: String) -> Variant:
	return config.get(KEYS[key][0])


func _apply_setting(params: Dictionary) -> Dictionary:
	var key := RnReqUtil.str_param(params, "key")
	if not KEYS.has(key):
		return RnReqUtil.fail("unknown setting " + key)
	var value: Variant = _coerce(params.get("value"), KEYS[key][1])
	if value == null:
		return RnReqUtil.fail("invalid value for " + key)
	var config := Global.get_config()
	if key == "locale":
		LocaleSettings.set_locale(value)
	else:
		config.set(KEYS[key][0], value)
		_apply(key, value)
	if bool(params.get("commit", false)) or key == "locale":
		config.save_to_settings_file()
	_announce(key)
	if key == "graphic_profile":
		for dependent: String in PROFILE_DEPENDENT:
			_announce(dependent)
	return RnReqUtil.ok({"key": key, "value": _read(config, key)})


static func _coerce(value: Variant, kind: String) -> Variant:
	match kind:
		"bool":
			return value if value is bool else null
		"string":
			return value if value is String else null
		"int":
			return int(value) if value is float or value is int else null
		"float":
			return float(value) if value is float or value is int else null
	return null


func _apply(key: String, value: Variant) -> void:
	match key:
		"graphic_profile":
			if int(value) < ConfigData.PROFILE_CUSTOM:
				GraphicSettings.apply_graphic_profile(int(value))
			Global.dynamic_graphics_manager.on_manual_profile_change(int(value))
		"dynamic_graphics_enabled":
			Global.dynamic_graphics_manager.set_enabled(bool(value))
		"view_distance":
			if is_instance_valid(Global.player_camera_node):
				Global.player_camera_node.far = float(value)
		"particle_quality":
			AvatarAnimHelpers.apply_particles_enabled(int(value) > 0)
			GraphicSettings.apply_particle_quality(int(value))
		"limit_fps":
			GraphicSettings.apply_fps_limit()
		"max_cache_size":
			GeneralSettings.apply_max_cache_size()
		"audio_general_volume":
			AudioSettings.apply_general_volume_settings()
		"audio_scene_volume":
			AudioSettings.apply_scene_volume_settings()
		"audio_voice_chat_volume":
			AudioSettings.apply_voice_chat_volume_settings()
		"audio_ui_volume":
			AudioSettings.apply_ui_volume_settings()
		"audio_music_volume":
			AudioSettings.apply_music_volume_settings()
		"audio_avatar_and_emotes_volume":
			AudioSettings.apply_avatar_and_emotes_volume_settings()
		"audio_mic_amplification":
			AudioSettings.apply_mic_amplification_settings()


func _announce(key: String) -> void:
	_emit.call("settings_changed", {"key": key, "value": _read(Global.get_config(), key)})


func _cache_size() -> Dictionary:
	var index := clampi(Global.get_config().max_cache_size, 0, CACHE_SIZE_MB.size() - 1)
	return {
		"bytes": Global.content_provider.get_cache_folder_total_size(),
		"max_bytes": CACHE_SIZE_MB[index] * BYTES_PER_MB,
	}


func _async_clear_cache() -> Dictionary:
	await PromiseUtils.async_awaiter(Global.content_provider.clear_cache_folder())
	return RnReqUtil.ok({"bytes": Global.content_provider.get_cache_folder_total_size()})


func _display_version() -> String:
	if Global.is_production():
		var parts: PackedStringArray = DclGlobal.get_short_version().split(".")
		if parts.size() >= 3:
			return "%s.%s.%s" % [parts[0], parts[1], parts[2]]
		return DclGlobal.get_short_version()
	return DclGlobal.get_full_version()
