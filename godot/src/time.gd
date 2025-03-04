extends Node

# Time syncing singleton for the sky shader

const TIME_OFFSET = .43

var config: DclConfig
var last_value: float


func _ready():
	config = Global.get_config()


func _process(_delta: float) -> void:
	var time = DclGlobalTime.get_world_time() if config.dynamic_skybox else config.skybox_time
	var normalized_time = float(time) / 86400
	normalized_time += TIME_OFFSET
	normalized_time -= floor(normalized_time)

	if last_value != normalized_time:
		RenderingServer.global_shader_parameter_set("day_night_cycle", normalized_time)
		last_value = normalized_time
