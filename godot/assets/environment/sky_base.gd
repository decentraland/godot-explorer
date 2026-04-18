class_name SkyBase
extends Node

# External gradient resources for time-of-day lighting
@export var directional_light_gradient: Gradient
@export var ambient_light_gradient: Gradient
@export var fog_color_gradient: Gradient

# Curve resources for sun/moon sky rendering
@export var sun_opacity_curve: Curve
@export var sun_size_curve: Curve
@export var moon_mask_size_curve: Curve

# Debug: when > 0, override Global.skybox_time with a fast cycle of N seconds for the
# whole day. Useful for verifying day/night transitions without waiting 24 in-game hours.
# Set to 0 in production.
@export var debug_cycle_seconds: float = 0.0

var _moon_smooth_dir := Vector3(0.0, 1.0, 0.0)

@onready var world_environment: WorldEnvironment = $WorldEnvironment
@onready var main_light: DirectionalLight3D = $SkyLights/MainLight
@onready var anim_player: AnimationPlayer = $SkyLights/AnimationPlayer
@onready var initial_sun_energy = main_light.light_energy
@onready var sky_material = world_environment.environment.sky.sky_material


func _ready():
	# Set up animation: play, sample at position, stop
	anim_player.play("light_cycle")
	anim_player.pause()

	if Global.is_xr():
		Global.loading_started.connect(self._on_loading_started)
		Global.loading_finished.connect(self._on_loading_finished)


func on_scene_runner_child_entered_tree(node: Node3D):
	node.hide()
	prints("Hiding:", node.name)


func _on_loading_started():
	world_environment.environment.background_energy_multiplier = 0.0
	world_environment.environment.ambient_light_energy = 0.0
	main_light.light_energy = 0.0

	var scene_runner = Global.get_scene_runner()
	scene_runner.child_entered_tree.connect(self.on_scene_runner_child_entered_tree)
	for child in scene_runner.get_children():
		child.hide()


func _on_loading_finished():
	var scene_runner = Global.get_scene_runner()
	scene_runner.child_entered_tree.disconnect(self.on_scene_runner_child_entered_tree)
	for child in scene_runner.get_children():
		child.show()
	var tween = get_tree().create_tween().set_parallel(true)
	world_environment.environment.background_energy_multiplier = 0.0
	world_environment.environment.ambient_light_energy = 0.0
	main_light.light_energy = 0.0

	tween.tween_property(world_environment, "environment:background_energy_multiplier", 1.0, 1.0)
	tween.tween_property(world_environment, "environment:ambient_light_energy", 1.0, 1.0)
	tween.tween_property(main_light, "light_energy", initial_sun_energy, 1.0)


func _process(_delta: float) -> void:
	var skybox_time: float
	if debug_cycle_seconds > 0.0:
		skybox_time = fmod(Time.get_ticks_msec() / (debug_cycle_seconds * 1000.0), 1.0)
	else:
		skybox_time = Global.skybox_time.get_normalized_time()

	# Sample the imported 144-keyframe sun rotation animation at current time
	anim_player.seek(skybox_time, true)

	# Drive day_night_cycle global so the shader's cloud color, sun/moon color, and floor
	# gradients all advance with skybox_time. The shader derives the baked CubemapArray
	# layer indices and blend factor from this same global, so no extra push needed.
	RenderingServer.global_shader_parameter_set("day_night_cycle", skybox_time)

	# Energy from light elevation (positive when sun above horizon)
	var light_dir = -main_light.global_transform.basis.z
	var elevation = -light_dir.y
	var energy_factor = smoothstep(-0.05, 0.3, elevation)

	main_light.visible = energy_factor > 0.01
	main_light.light_energy = initial_sun_energy * energy_factor

	# Visual sun direction — driven by the animation.
	var sun_dir = main_light.global_transform.basis.z
	RenderingServer.global_shader_parameter_set("sun_direction", sun_dir)

	# Atmospheric sun position — keeps the visual sun's azimuth (X/Z) so the bake's bright
	# halo lines up with where the visual sun disc appears, but synthesizes the Y component
	# so the sun cycles below horizon at night (the animation's Y stays mostly positive,
	# which would never produce a dark Rayleigh sky).
	var atm_sun_dir = Vector3(sun_dir.x, -cos(TAU * skybox_time), sun_dir.z).normalized()
	RenderingServer.global_shader_parameter_set("atm_sun_direction", atm_sun_dir)

	# Moon = opposite of visual sun. The sun stays mostly above horizon in the animation,
	# so -sun_dir puts the moon mostly below — only briefly above during dusk/dawn.
	# Visibility is gated by moon_mask_size_curve. Slerp filters dusk/dawn jumps.
	var moon_target = -sun_dir
	_moon_smooth_dir = _moon_smooth_dir.slerp(moon_target, clampf(_delta * 3.0, 0.0, 1.0))
	RenderingServer.global_shader_parameter_set("moon_direction", _moon_smooth_dir)

	if sun_opacity_curve:
		RenderingServer.global_shader_parameter_set(
			"sun_opacity", sun_opacity_curve.sample(skybox_time)
		)
	if sun_size_curve:
		RenderingServer.global_shader_parameter_set("sun_size", sun_size_curve.sample(skybox_time))
	if moon_mask_size_curve:
		RenderingServer.global_shader_parameter_set(
			"moon_mask_size", moon_mask_size_curve.sample(skybox_time)
		)

	# Gradients drive light/ambient/fog color. Direct assignment — no .lerp(0.5) bug.
	# No early-out: the Curve sample above runs every frame anyway, so gating gradient
	# samples doesn't save anything and risks falling out of sync.
	if directional_light_gradient:
		main_light.light_color = directional_light_gradient.sample(skybox_time)
	if ambient_light_gradient:
		world_environment.environment.ambient_light_color = ambient_light_gradient.sample(
			skybox_time
		)
	if fog_color_gradient:
		world_environment.environment.fog_light_color = fog_color_gradient.sample(skybox_time)
