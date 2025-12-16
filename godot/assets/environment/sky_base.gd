class_name SkyBase
extends Node

# Day/night phase timing constants (normalized 0.0-1.0)
# Sun is active for 70% of the day (0.15-0.85), moon for 30% (0.85-0.15 wrapping)
const SUNRISE_START = 0.12  # Dawn begins
const SUNRISE_END = 0.15    # Sun fully up, moon down
const SUNSET_START = 0.85   # Dusk begins
const SUNSET_END = 0.88     # Moon fully up, sun down

# Derived constants for phase durations
const SUN_PHASE_DURATION = SUNSET_START - SUNRISE_END  # 0.7 (70% of day)
const MOON_PHASE_DURATION = 1.0 - SUN_PHASE_DURATION   # 0.3 (30% of day)

# Transition fade window (how far before/after phase change to fade light)
const TRANSITION_FADE_MARGIN = 0.03

# Horizon colors for transitions
@export var moon_horizon_color := Color("#ff7534")  # Orange
@export var sun_horizon_color := Color("#8f0025")  # Deep red

# External gradient resources for time-of-day lighting
@export var directional_light_gradient: Gradient
@export var ambient_light_gradient: Gradient
@export var fog_color_gradient: Gradient

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


func _process(_delta: float) -> void:
	var skybox_time = Global.skybox_time.get_normalized_time()

	# Calculate angular positions (0 = horizon rise, 0.5 = zenith, 1.0 = horizon set)
	var sun_angle: float
	var moon_angle: float

	# Sun phase: SUNRISE_END -> noon -> SUNSET_START
	if skybox_time >= SUNRISE_END and skybox_time < SUNSET_START:
		sun_angle = (skybox_time - SUNRISE_END) / SUN_PHASE_DURATION
	else:
		sun_angle = 0.0  # Below horizon

	# Moon phase: SUNSET_START -> midnight -> SUNRISE_END (wraps around)
	if skybox_time >= SUNSET_START:
		# Rising half: SUNSET_START to 1.0 maps to 0.0-0.5
		moon_angle = (skybox_time - SUNSET_START) / MOON_PHASE_DURATION
	elif skybox_time < SUNRISE_END:
		# Setting half: 0.0 to SUNRISE_END maps to 0.5-1.0
		moon_angle = 0.5 + (skybox_time / MOON_PHASE_DURATION)
	else:
		moon_angle = 0.0  # Below horizon

	# Calculate blend factor between sun and moon (0.0 = full sun, 1.0 = full moon)
	var sun_to_moon_blend: float
	if skybox_time < SUNRISE_START:
		# Deep night - full moon
		sun_to_moon_blend = 1.0
	elif skybox_time < SUNRISE_END:
		# Dawn transition - moon to sun
		sun_to_moon_blend = 1.0 - smoothstep(SUNRISE_START, SUNRISE_END, skybox_time)
	elif skybox_time < SUNSET_START:
		# Day - full sun
		sun_to_moon_blend = 0.0
	elif skybox_time < SUNSET_END:
		# Dusk transition - sun to moon
		sun_to_moon_blend = smoothstep(SUNSET_START, SUNSET_END, skybox_time)
	else:
		# Night - full moon
		sun_to_moon_blend = 1.0

	# Determine which phase we're in (snap, not blend for rotation/transform)
	var is_sun_phase = skybox_time >= SUNRISE_END and skybox_time < SUNSET_START

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

	# Fade light to zero near phase transitions to hide rotation snap
	var transition_fade = 1.0
	if skybox_time >= SUNRISE_START and skybox_time < SUNRISE_END + TRANSITION_FADE_MARGIN:
		# Dawn transition - fade centered around SUNRISE_END
		var dist = abs(skybox_time - SUNRISE_END)
		transition_fade = smoothstep(0.0, TRANSITION_FADE_MARGIN, dist)
	elif skybox_time >= SUNSET_START - TRANSITION_FADE_MARGIN and skybox_time < SUNSET_END:
		# Dusk transition - fade centered around SUNSET_START
		var dist = abs(skybox_time - SUNSET_START)
		transition_fade = smoothstep(0.0, TRANSITION_FADE_MARGIN, dist)

	# Apply transformations
	main_light.visible = transition_fade > 0.01
	main_light.light_energy = current_energy * t * transition_fade

	# Rotate light through full arc based on angle (0 = rising, 0.5 = zenith, 1.0 = setting)
	# Map angle from 0-1 to a rotation - negate to flip direction
	var rotation_angle = -(current_angle - 0.5) * PI  # Flip: PI/2 to -PI/2
	main_light.global_transform = current_transform.rotated(Vector3(1.0, 0.0, 0.0), rotation_angle)

	main_light.light_color = lerp(current_horizon_color, current_color, t)

	# Update colors based on time of day using gradients
	if last_time != skybox_time:
		last_time = skybox_time

		if directional_light_gradient:
			var gradient_color = directional_light_gradient.sample(skybox_time)
			main_light.light_color = main_light.light_color.lerp(gradient_color, 0.5)

		if ambient_light_gradient:
			world_environment.environment.ambient_light_color = ambient_light_gradient.sample(skybox_time)

		if fog_color_gradient:
			world_environment.environment.fog_light_color = fog_color_gradient.sample(skybox_time)
