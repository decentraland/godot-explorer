extends Node
class_name EnvironmentSelector

var sky: Node = null


func _ready():
	set_skybox(Global.config.skybox)

func set_quality_level(level: int):
	var soft_shadow_filter_quality = 0
	if level == 0: # Low
		soft_shadow_filter_quality = 0
	elif level == 1: # Medium
		soft_shadow_filter_quality = 1
	elif level == 2: # High
		soft_shadow_filter_quality = 4
		
	ProjectSettings.set_setting("rendering/lights_and_shadows/directional_shadow/soft_shadow_filter_quality", soft_shadow_filter_quality)
	ProjectSettings.save()
	set_skybox(level)

func set_skybox(skybox_index: int):
	if sky != null:
		sky.queue_free()

	if Global.testing_scene_mode:
		sky = load("res://assets/sky/sky_test.tscn").instantiate()
	else:
		match skybox_index:
			0:
				sky = load("res://assets/environment/sky_low/sky_low.tscn").instantiate()
			1:
				sky = load("res://assets/environment/sky_medium/sky_medium.tscn").instantiate()
			2:
				sky = load("res://assets/environment/sky_high/sky_high.tscn").instantiate()

	add_child(sky)
