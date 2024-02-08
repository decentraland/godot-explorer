class_name AudioSettings extends RefCounted


static func apply_volume_settings():
	var bus_index := AudioServer.get_bus_index("Master")
	var general_db = -80.0 + (80.0 * (float(Global.config.audio_general_volume) / 100.0))
	AudioServer.set_bus_volume_db(bus_index, general_db)
