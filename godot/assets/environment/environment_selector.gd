class_name EnvironmentSelector
extends Node

var sky: SkyBase = null


func _ready():
	set_skybox(Global.get_config().skybox)
	set_anti_aliasing(Global.get_config().anti_aliasing)
	Global.get_config().param_changed.connect(self._on_config_changed)


func _on_config_changed(param: ConfigData.ConfigParams):
	set_skybox(Global.get_config().skybox)
	if param == ConfigData.ConfigParams.ANTI_ALIASING:
		set_anti_aliasing(Global.get_config().anti_aliasing)


func set_skybox(skybox_index: int):
	if sky != null:
		sky.queue_free()

	if Global.testing_scene_mode:
		sky = load("res://assets/environment/sky_high/sky_high.tscn").instantiate()
	else:
		match skybox_index:
			0:
				sky = load("res://assets/environment/sky_low/sky_low.tscn").instantiate()
			1:
				sky = load("res://assets/environment/sky_medium/sky_medium.tscn").instantiate()
			2:
				# gdlint:ignore = duplicated-load
				sky = load("res://assets/environment/sky_high/sky_high.tscn").instantiate()

	add_child(sky)


# Reason that anti aliasing is here it's because
# it applies to the viewport that is being rendered
# so it is the same than the environment that we want
func set_anti_aliasing(anti_aliasing: int):
	var value: RenderingServer.ViewportMSAA = RenderingServer.VIEWPORT_MSAA_DISABLED
	match Global.get_config().anti_aliasing:
		0:  # OFF
			value = RenderingServer.VIEWPORT_MSAA_DISABLED
		1:  # x2
			value = RenderingServer.VIEWPORT_MSAA_2X
		2:  # x4
			value = RenderingServer.VIEWPORT_MSAA_4X
		3:  # x8
			value = RenderingServer.VIEWPORT_MSAA_8X

	RenderingServer.viewport_set_msaa_3d(get_viewport().get_viewport_rid(), value)
