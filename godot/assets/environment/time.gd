class_name SkyboxTime
extends Node

# Time syncing singleton for the sky shader

# Transition speed: how fast to interpolate when SDK changes the time
const TRANSITION_SPEED: float = 0.5  # normalized time per second (0.5 = half day in 1 second)

# Debug mode - set to true to make a full day/night cycle every 10 seconds
@export var debug_time_rotation: bool = false

var config: DclConfig
var last_value: float
var normalized_time := 0.0
var debug_time_accumulator: float = 0.0
var target_time := 0.0
var is_transitioning := false
var transition_forward := true  # Direction of transition


func _ready():
	config = Global.get_config()


func get_normalized_time():
	return normalized_time


func _process(delta: float) -> void:
	# Debug mode: full cycle every 10 seconds
	if debug_time_rotation:
		debug_time_accumulator += delta
		normalized_time = fmod(debug_time_accumulator / 10.0, 1.0)
		if last_value != normalized_time:
			RenderingServer.global_shader_parameter_set("day_night_cycle", normalized_time)
			last_value = normalized_time
		return

	var scene_runner = Global.scene_runner

	# Check if SDK is controlling the skybox time
	if scene_runner.sdk_skybox_time_active:
		# SDK is controlling: use the fixed time from the scene
		target_time = float(scene_runner.sdk_skybox_fixed_time) / 86400.0
		transition_forward = scene_runner.sdk_skybox_transition_forward

		# Smooth transition to target time
		if not is_equal_approx(normalized_time, target_time):
			is_transitioning = true
			normalized_time = _interpolate_time(normalized_time, target_time, delta)
		else:
			is_transitioning = false
	elif Global.get_fixed_skybox_time():
		# Fixed skybox for testing
		target_time = 0.625  # 3pm
		normalized_time = target_time
		is_transitioning = false
	else:
		# Normal behavior: use config or world time
		if config.dynamic_skybox:
			target_time = float(DclGlobalTime.get_world_time()) / 86400.0
		else:
			target_time = float(config.skybox_time) / 86400.0

		# When not SDK controlled, transition back smoothly if needed
		if is_transitioning:
			normalized_time = _interpolate_time(normalized_time, target_time, delta)
			if is_equal_approx(normalized_time, target_time):
				is_transitioning = false
		else:
			normalized_time = target_time

	if last_value != normalized_time:
		RenderingServer.global_shader_parameter_set("day_night_cycle", normalized_time)
		last_value = normalized_time


func _interpolate_time(current: float, target: float, delta: float) -> float:
	# Calculate the shortest path considering wrapping at 0/1
	var diff = target - current

	# If transitioning backward, we might need to go the "long way"
	if not transition_forward:
		# For backward transitions, prefer going backwards (decreasing time)
		if diff > 0:
			diff -= 1.0  # Go backwards through midnight
	else:
		# For forward transitions, prefer going forwards (increasing time)
		if diff < 0:
			diff += 1.0  # Go forward through midnight

	# Calculate step based on transition speed
	var step = TRANSITION_SPEED * delta

	if abs(diff) <= step:
		return target

	var new_time = current + sign(diff) * step

	# Wrap around [0, 1]
	if new_time < 0.0:
		new_time += 1.0
	elif new_time >= 1.0:
		new_time -= 1.0

	return new_time
