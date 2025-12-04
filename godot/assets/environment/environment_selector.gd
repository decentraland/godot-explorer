class_name EnvironmentSelector
extends Node

var sky: SkyBase = null


func _ready():
	set_skybox_and_shadow(Global.get_config().skybox)
	set_anti_aliasing(Global.get_config().anti_aliasing)
	set_bloom(Global.get_config().bloom_quality)
	Global.get_config().param_changed.connect(self._on_config_changed)


func _on_config_changed(param: ConfigData.ConfigParams):
	if param == ConfigData.ConfigParams.SKY_BOX:
		set_skybox_and_shadow(Global.get_config().skybox)
	elif param == ConfigData.ConfigParams.SHADOW_QUALITY:
		set_shadow(Global.get_config().shadow_quality)
	elif param == ConfigData.ConfigParams.BLOOM_QUALITY:
		set_bloom(Global.get_config().bloom_quality)
	elif param == ConfigData.ConfigParams.ANTI_ALIASING:
		set_anti_aliasing(Global.get_config().anti_aliasing)


func set_skybox_and_shadow(skybox_index: int):
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
	set_shadow(Global.get_config().shadow_quality)


func set_shadow(shadow_quality: int):
	var quality: RenderingServer.ShadowQuality = RenderingServer.SHADOW_QUALITY_HARD
	match shadow_quality:
		0:  # no shadow
			sky.main_light.shadow_enabled = false
			# Increase light energy when shadows are off to compensate
			sky.main_light.light_energy = 1.0
		1:  # low res shadow
			sky.main_light.shadow_enabled = true
			# Use base light energy when shadows are on
			sky.main_light.light_energy = 0.7
		2:  # high res shadow
			sky.main_light.shadow_enabled = true
			sky.main_light.light_energy = 0.7
			quality = RenderingServer.SHADOW_QUALITY_SOFT_MEDIUM

	RenderingServer.directional_soft_shadow_filter_set_quality(quality)


func set_bloom(bloom_quality: int):
	var environment = sky.world_environment.environment
	match bloom_quality:
		0:  # Off
			environment.glow_enabled = false
		1:  # Low quality
			environment.glow_enabled = true
			environment.glow_intensity = 0.7
			environment.set("glow_levels/1", 0.2)
			environment.set("glow_levels/2", 0.15)
			environment.set("glow_levels/3", 0.0)
			environment.set("glow_levels/5", 0.0)
		2:  # High quality
			environment.glow_enabled = true
			environment.glow_intensity = 1.5
			environment.set("glow_levels/1", 0.4)
			environment.set("glow_levels/2", 0.3)
			environment.set("glow_levels/3", 0.0)
			environment.set("glow_levels/5", 0.0)


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
