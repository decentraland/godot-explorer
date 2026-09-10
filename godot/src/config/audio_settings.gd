class_name AudioSettings extends RefCounted

const MIN_VOLUME_DB := -80.0


## Slider position (0-100) to bus gain. Linear in amplitude, like Unity's
## `AudioUtils.PercentageVolumeToDecibel`: 50% is -6 dB, not -40 dB.
static func percentage_to_db(percentage: float) -> float:
	if percentage <= 0.0:
		return MIN_VOLUME_DB
	return maxf(MIN_VOLUME_DB, linear_to_db(percentage / 100.0))


static func _apply_bus_percentage(bus_name: StringName, percentage: float) -> void:
	var bus_index := AudioServer.get_bus_index(bus_name)
	if bus_index < 0:
		printerr("AudioSettings: unknown audio bus ", bus_name)
		return
	AudioServer.set_bus_volume_db(bus_index, percentage_to_db(percentage))


static func apply_volume_settings():
	apply_general_volume_settings()
	apply_scene_volume_settings()
	apply_ui_volume_settings()
	apply_music_volume_settings()
	apply_avatar_and_emotes_volume_settings()
	apply_voice_chat_volume_settings()
	apply_mic_amplification_settings()


static func apply_general_volume_settings():
	_apply_bus_percentage(&"Master", Global.get_config().audio_general_volume)


static func apply_scene_volume_settings(force_value = null):
	var scene_volume: float = Global.get_config().audio_scene_volume
	if force_value is float:
		scene_volume = force_value

	_apply_bus_percentage(&"Scene", scene_volume)


static func apply_voice_chat_volume_settings(force_value = null):
	var voice_volume: float = Global.get_config().audio_voice_chat_volume
	if force_value is float:
		voice_volume = force_value

	_apply_bus_percentage(&"VoiceChat", voice_volume)


static func apply_ui_volume_settings():
	_apply_bus_percentage(&"UI", Global.get_config().audio_ui_volume)


static func apply_music_volume_settings():
	_apply_bus_percentage(&"Music", Global.get_config().audio_music_volume)


static func apply_avatar_and_emotes_volume_settings():
	_apply_bus_percentage(&"AvatarAndEmotes", Global.get_config().audio_avatar_and_emotes_volume)


static func apply_mic_amplification_settings():
	pass
