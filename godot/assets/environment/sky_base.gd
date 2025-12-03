class_name SkyBase
extends Node

# Time origins for sun and moon (normalized 0.0-1.0)
const SUN_ORIGIN = 0.32  # ~7:40 AM
const MOON_ORIGIN = 0.82  # ~7:40 PM

# Horizon colors for transitions
@export var moon_horizon_color := Color("#ff7534")  # Orange
@export var sun_horizon_color := Color("#8f0025")  # Deep red

var last_time := 0.0

# Moon properties (night time) - hardcoded since we only have one physical light
var initial_moon_energy: float = 0.3
var initial_moon_color: Color = Color(0.77, 0.992333, 1, 1)
var initial_moon_transform: Transform3D

@onready var world_environment: WorldEnvironment = $WorldEnvironment
@onready var main_light: DirectionalLight3D = $SkyLights/MainLight

# Sun properties (day time)
@onready var initial_sun_energy = main_light.light_energy
@onready var initial_sun_transform = main_light.global_transform
@onready var initial_sun_color = main_light.light_color

@onready var sky_material = world_environment.environment.sky.sky_material
@onready
var day_fog_color = sky_material.get_shader_parameter("clouds_gradient_day").gradient.colors[0]
@onready
var night_fog_color = sky_material.get_shader_parameter("clouds_gradient_night").gradient.colors[0]


func _ready():
	# Calculate moon transform (opposite side of sky from sun)
	initial_moon_transform = initial_sun_transform.rotated(Vector3(0.0, 1.0, 0.0), PI)

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


# Determine if we're in sun or moon phase
func get_phase_blend(normalized_time: float) -> float:
	# Returns 0.0 for full sun, 1.0 for full moon
	# Transitions happen around dawn (0.07) and dusk (0.57)
	var cycle = fmod(normalized_time + 0.5 - SUN_ORIGIN, 1.0)

	# Sun is dominant from 0.0 to 0.5, moon from 0.5 to 1.0
	if cycle < 0.5:
		return 0.0  # Sun phase
	return 1.0  # Moon phase


# Get the transition factor (0.0 = full brightness, 1.0 = transitioning/faded)
func get_transition_fade(normalized_time: float) -> float:
	# Calculate angular position for both sun and moon
	var sun_time = 1.0 + normalized_time
	var sun_angle = clamp(((sun_time - SUN_ORIGIN) - floor(sun_time - SUN_ORIGIN)) * 2.0, 0.0, 1.0)

	var moon_time = 1.0 + normalized_time
	var moon_angle = clamp(
		((moon_time - MOON_ORIGIN) - floor(moon_time - MOON_ORIGIN)) * 2.0, 0.0, 1.0
	)

	# Near horizon = high fade value (close to 0 or 1)
	# Overhead = low fade value (close to 0.5)
	var sun_fade = 1.0 - (smoothstep(0.0, 0.15, sun_angle) * smoothstep(1.0, 0.85, sun_angle))
	var moon_fade = 1.0 - (smoothstep(0.0, 0.15, moon_angle) * smoothstep(1.0, 0.85, moon_angle))

	# Return the minimum (we want to fade when either is transitioning)
	return min(sun_fade, moon_fade)


func _process(_delta: float) -> void:
	var skybox_time = Global.skybox_time.get_normalized_time()

	# Calculate angular positions (0 = horizon rise, 0.5 = zenith, 1.0 = horizon set)
	var sun_angle: float
	var moon_angle: float

	# Sun phase: 0.25 (sunrise) -> 0.5 (noon) -> 0.75 (sunset)
	if skybox_time >= 0.25 and skybox_time < 0.75:
		# Map 0.25-0.75 to 0.0-1.0
		sun_angle = (skybox_time - 0.25) / 0.5
	else:
		sun_angle = 0.0  # Below horizon

	# Moon phase: 0.75 (moonrise) -> 0.0 (midnight) -> 0.25 (moonset)
	if skybox_time >= 0.75:
		# Map 0.75-1.0 to 0.0-0.5
		moon_angle = (skybox_time - 0.75) / 0.5
	elif skybox_time < 0.25:
		# Map 0.0-0.25 to 0.5-1.0
		moon_angle = 0.5 + (skybox_time / 0.5)
	else:
		moon_angle = 0.0  # Below horizon

	# Calculate blend factor between sun and moon (0.0 = full sun, 1.0 = full moon)
	# Smooth transition during dusk (0.7-0.8) and dawn (0.2-0.3)
	var sun_to_moon_blend: float
	if skybox_time < 0.2:
		# Early morning - full moon
		sun_to_moon_blend = 1.0
	elif skybox_time < 0.3:
		# Dawn transition - moon to sun
		sun_to_moon_blend = 1.0 - smoothstep(0.2, 0.3, skybox_time)
	elif skybox_time < 0.7:
		# Day - full sun
		sun_to_moon_blend = 0.0
	elif skybox_time < 0.8:
		# Dusk transition - sun to moon
		sun_to_moon_blend = smoothstep(0.7, 0.8, skybox_time)
	else:
		# Night - full moon
		sun_to_moon_blend = 1.0

	# Determine which phase we're in (snap, not blend for rotation/transform)
	# Switch happens at the very end/start of transitions when light is almost off
	var is_sun_phase = skybox_time >= 0.25 and skybox_time < 0.75

	# Rotation/transform: snap to sun or moon (no blending)
	var current_angle: float
	var current_transform: Transform3D
	if is_sun_phase:
		current_angle = sun_angle
		current_transform = initial_sun_transform
	else:
		current_angle = moon_angle
		current_transform = initial_moon_transform

	# Color and energy: blend smoothly for visual transitions
	var current_color = initial_sun_color.lerp(initial_moon_color, sun_to_moon_blend)
	var current_horizon_color = sun_horizon_color.lerp(moon_horizon_color, sun_to_moon_blend)
	var current_energy = lerp(initial_sun_energy, initial_moon_energy, sun_to_moon_blend)

	# Calculate brightness based on angle (0=horizon, 0.5=overhead, 1.0=horizon)
	var t = smoothstep(0.0, 0.2, current_angle) * smoothstep(1.0, 0.8, current_angle)

	# Fade light to zero near phase transitions (0.25 and 0.75) to hide rotation snap
	# Sharper fade with less time fully off
	var transition_fade = 1.0
	if skybox_time >= 0.24 and skybox_time < 0.26:
		# Dawn transition - quick fade out/in around 0.25
		var dist = abs(skybox_time - 0.25)
		transition_fade = smoothstep(0.0, 0.01, dist)  # Sharper: 0.01 instead of 0.02
	elif skybox_time >= 0.74 and skybox_time < 0.76:
		# Dusk transition - quick fade out/in around 0.75
		var dist = abs(skybox_time - 0.75)
		transition_fade = smoothstep(0.0, 0.01, dist)  # Sharper: 0.01 instead of 0.02

	# Apply transformations
	main_light.visible = transition_fade > 0.01
	main_light.light_energy = current_energy * t * transition_fade

	# Rotate light through full arc based on angle (0 = rising, 0.5 = zenith, 1.0 = setting)
	# Map angle from 0-1 to a rotation - negate to flip direction
	var rotation_angle = -(current_angle - 0.5) * PI  # Flip: PI/2 to -PI/2
	main_light.global_transform = current_transform.rotated(Vector3(1.0, 0.0, 0.0), rotation_angle)

	main_light.light_color = lerp(current_horizon_color, current_color, t)

	# Dynamic ambient light - boost when it's night OR near horizon
	# Base: 0.6 (increased from 0.05 for brighter appearance)
	var base_ambient = 0.6

	# Calculate horizon factor (0.0 = zenith, 1.0 = horizon)
	var horizon_factor = 0.0
	if current_angle < 0.3:
		# Rising - approaching horizon
		horizon_factor = 1.0 - smoothstep(0.0, 0.3, current_angle)
	elif current_angle > 0.7:
		# Setting - approaching horizon
		horizon_factor = smoothstep(0.7, 1.0, current_angle)

	# Boost ambient when night OR horizon (use max to get OR behavior)
	var night_factor = sun_to_moon_blend  # 0.0 = day, 1.0 = night
	var boost_factor = max(night_factor, horizon_factor)  # OR: take whichever is higher

	var ambient_boost = boost_factor * 0.4  # Up to 0.4 extra (increased from 0.15)
	var ambient_energy = base_ambient + ambient_boost
	world_environment.environment.ambient_light_energy = ambient_energy

	# Update fog color based on time of day
	if last_time != skybox_time:
		last_time = skybox_time
		var fog_cycle = skybox_time + .43
		fog_cycle -= floor(fog_cycle)
		var blend = sin(fog_cycle * PI)
		world_environment.environment.fog_light_color = day_fog_color.lerp(night_fog_color, blend)
