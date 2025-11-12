class_name SkyboxTime
extends Node

# Time syncing singleton for the sky shader

var config: DclConfig
var last_value: float

var normalized_time := 0.0
var debug_mode_active := false  # Flag to disable normal time updates when in debug mode


func _ready():
	config = Global.get_config()


func get_normalized_time():
	return normalized_time


func _process(_delta: float) -> void:
	# Skip normal time updates if debug mode is active
	if debug_mode_active:
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
