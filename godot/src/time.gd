extends Node

# Time syncing singleton for the sky shader

var config: DclConfig
var last_value: float

var normalized_time := 0.0

func _ready():
	config = Global.get_config()

var t=.0
func _process(_delta: float) -> void:
	t+=_delta*.1
	# normalized_time = float(((t-floor(t))*86400) if config.dynamic_skybox else config.skybox_time) / 86400
	normalized_time = float(DclGlobalTime.get_world_time() if config.dynamic_skybox else config.skybox_time) / 86400
	if last_value != normalized_time:
		RenderingServer.global_shader_parameter_set("day_night_cycle", normalized_time)
		last_value = normalized_time
