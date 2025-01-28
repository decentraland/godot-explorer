extends Node

# Time syncing singleton for the sky shader

const TIME_OFFSET = .43

func _process(_delta: float) -> void:
	var time = int(floor(Time.get_unix_time_from_system()) + Time.get_time_zone_from_system().bias * 60) % 86400 
	var normalized_time =  float(time)/ 86400
	normalized_time += TIME_OFFSET
	normalized_time -= floor(normalized_time)
	RenderingServer.global_shader_parameter_set("day_night_cycle", normalized_time)
