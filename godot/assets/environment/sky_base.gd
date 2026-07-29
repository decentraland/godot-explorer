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

# Moon (night fill) light. The baked light_cycle animation doubles as a moon
# trajectory on its night arc (~21:00 → ~05:00), so MoonLight just copies the
# MainLight rotation and ramps energy/color with the moon factor.
@export var moon_light_color: Color = Color(0.55, 0.65, 0.95)
@export var moon_light_energy: float = 0.9

# Minimum sun elevation (degrees) during daytime. Below this the key light
# grazes the horizon and avatars/objects lose their ground shadow (issue
# #2516). Only the light is clamped; the visual sun in the sky shader keeps
# following the baked animation.
@export var min_sun_elevation_degrees: float = 12.0

var _moon_smooth_dir := Vector3(0.0, 1.0, 0.0)

@onready var world_environment: WorldEnvironment = $WorldEnvironment
@onready var main_light: DirectionalLight3D = $SkyLights/MainLight
@onready var moon_light: DirectionalLight3D = get_node_or_null("SkyLights/MoonLight")
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

	# 0.0 during the day, 1.0 when the baked animation is on its night (moon)
	# arc. Crossfades the sun light out and the moon light in around dusk/dawn.
	var moon_factor := SkyCycleMath.compute_moon_factor(skybox_time)

	# The baked dusk whip (light teleports from sunset parking to the night
	# arc, ~19:55-21:04) must stay dark — no light may follow that sweep.
	var lights_out := SkyCycleMath.is_lights_out(skybox_time)

	# The animation's sampled rotation before any correction/clamping.
	var animated_basis := main_light.global_transform.basis

	# The baked keyframes came from Unity WITHOUT the handedness correction
	# (180° around Y) — only the sky shader's celestial disc was compensated.
	# Apply the same correction to the lights themselves, so shadows fall AWAY
	# from the visible sun/moon instead of towards it (#2516).
	var light_basis := animated_basis.rotated(Vector3.UP, PI)

	# Energy from light elevation (positive when sun above horizon). The 180°
	# correction preserves Y, so elevation is unaffected.
	var light_dir = -light_basis.z
	var elevation = -light_dir.y
	var energy_factor := SkyCycleMath.compute_sun_energy_factor(elevation)

	main_light.global_transform.basis = light_basis

	# Daytime only: keep the key light from grazing the horizon so it always
	# casts a readable ground shadow (issue #2516). The visual sun drawn by the
	# sky shader still follows the corrected direction.
	if moon_factor < 1.0 and elevation > 0.0:
		var clamped_sun_dir := SkyCycleMath.clamp_direction_elevation(
			-light_dir, deg_to_rad(min_sun_elevation_degrees)
		)
		if clamped_sun_dir != -light_dir:
			main_light.global_transform.basis = Basis.looking_at(-clamped_sun_dir, Vector3.UP)

	# The sun light hands over to the moon light at night (moon_factor). The
	# sun gate keeps it off while the animation is on the night arc —
	# otherwise it would flash warm light from the moonset position at ~4am.
	var sun_factor := (
		energy_factor * (1.0 - moon_factor) * SkyCycleMath.compute_sun_gate(skybox_time)
	)
	main_light.visible = sun_factor > 0.01 and not lights_out
	main_light.light_energy = initial_sun_energy * sun_factor

	# Moon light: cool fill following the corrected night arc, so night scenes
	# keep a directional contribution instead of collapsing to ambient-only
	# black. Energy is purely time-driven: the fade-in starts once the arc is
	# stable (post-whip), the fade-out ends exactly at the horizon crossing,
	# so it never cuts abruptly nor uplights from below the horizon (#2516).
	if moon_light != null:
		moon_light.visible = moon_factor > 0.01 and not lights_out
		if moon_light.visible:
			moon_light.global_transform.basis = light_basis
			moon_light.light_energy = moon_light_energy * moon_factor
			moon_light.light_color = moon_light_color

	# Visual sun direction — the corrected light direction, which is exactly
	# where the sky shader draws the celestial disc (the shader-side 180°
	# mirror that used to compensate is now applied to the lights).
	var sun_dir = light_basis.z
	RenderingServer.global_shader_parameter_set("sun_direction", sun_dir)

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
		var sun_col := directional_light_gradient.sample(skybox_time)
		main_light.light_color = sun_col
		# Mirror the effective key-light color (sun by day, moon by night) to a
		# global shader param so unshaded materials (e.g. the octahedral
		# impostor) can match the day/night cycle.
		var key_col := sun_col.lerp(moon_light_color, moon_factor)
		RenderingServer.global_shader_parameter_set(
			"sun_color", Vector3(key_col.r, key_col.g, key_col.b)
		)
	if ambient_light_gradient:
		var amb_col := ambient_light_gradient.sample(skybox_time)
		world_environment.environment.ambient_light_color = amb_col
		RenderingServer.global_shader_parameter_set(
			"ambient_color", Vector3(amb_col.r, amb_col.g, amb_col.b)
		)
	if fog_color_gradient:
		world_environment.environment.fog_light_color = fog_color_gradient.sample(skybox_time)
