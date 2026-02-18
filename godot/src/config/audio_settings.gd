class_name AudioSettings extends RefCounted


static func apply_volume_settings():
	apply_general_volume_settings()
	apply_scene_volume_settings()
	apply_ui_volume_settings()
	apply_music_volume_settings()
	apply_avatar_and_emotes_volume_settings()
	apply_voice_chat_volume_settings()
	apply_mic_amplification_settings()


static func apply_general_volume_settings():
	var bus_index := AudioServer.get_bus_index("Master")
	var general_db = -80.0 + (80.0 * (float(Global.get_config().audio_general_volume) / 100.0))
	AudioServer.set_bus_volume_db(bus_index, general_db)


static func apply_scene_volume_settings(force_value = null):
	var scene_volume: float = Global.get_config().audio_scene_volume
	if force_value is float:
		scene_volume = force_value

	var bus_index := AudioServer.get_bus_index("Scene")
	var general_db = -80.0 + (80.0 * (float(scene_volume) / 100.0))
	AudioServer.set_bus_volume_db(bus_index, general_db)


static func apply_voice_chat_volume_settings(force_value = null):
	var voice_volume: float = Global.get_config().audio_voice_chat_volume
	if force_value is float:
		voice_volume = force_value

	var bus_index := AudioServer.get_bus_index("VoiceChat")
	var general_db = -80.0 + (80.0 * (float(voice_volume) / 100.0))
	AudioServer.set_bus_volume_db(bus_index, general_db)


static func apply_ui_volume_settings():
	var bus_index := AudioServer.get_bus_index("UI")
	var general_db = -80.0 + (80.0 * (float(Global.get_config().audio_ui_volume) / 100.0))
	AudioServer.set_bus_volume_db(bus_index, general_db)


static func apply_music_volume_settings():
	var bus_index := AudioServer.get_bus_index("Music")
	var general_db = -80.0 + (80.0 * (float(Global.get_config().audio_music_volume) / 100.0))
	AudioServer.set_bus_volume_db(bus_index, general_db)


static func apply_avatar_and_emotes_volume_settings():
	var bus_index := AudioServer.get_bus_index("AvatarAndEmotes")
	var general_db = (
		-80.0 + (80.0 * (float(Global.get_config().audio_avatar_and_emotes_volume) / 100.0))
	)
	AudioServer.set_bus_volume_db(bus_index, general_db)


static func apply_mic_amplification_settings():
	pass
