class_name SkyboxTime
extends Node

# Time syncing singleton for the sky shader

var config: DclConfig
var last_value: float

var normalized_time := 0.0

# Debug mode - set to true to make a full day/night cycle every 10 seconds
@export var debug_time_rotation: bool = true
var debug_time_accumulator: float = 0.0


func _ready():
	config = Global.get_config()


func get_normalized_time():
	return normalized_time


func _process(_delta: float) -> void:
	# Debug mode: full cycle every 10 seconds
	if debug_time_rotation:
		debug_time_accumulator += _delta
		normalized_time = fmod(debug_time_accumulator / 10.0, 1.0)
		if last_value != normalized_time:
			RenderingServer.global_shader_parameter_set("day_night_cycle", normalized_time)
			last_value = normalized_time
		return

	if Global.get_fixed_skybox_time():
		normalized_time = 0.625  # 3pm
	else:
		normalized_time = (
			float(DclGlobalTime.get_world_time() if config.dynamic_skybox else config.skybox_time)
			/ 86400
		)
	if last_value != normalized_time:
		RenderingServer.global_shader_parameter_set("day_night_cycle", normalized_time)
		last_value = normalized_time
