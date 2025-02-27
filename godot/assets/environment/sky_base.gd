class_name SkyBase
extends Node

@onready var world_environment: WorldEnvironment = $WorldEnvironment
@onready var sun_light: DirectionalLight3D = $SunLight
@onready var moon_light: DirectionalLight3D = $MoonLight

@onready var initial_sun_energy = sun_light.light_energy
@onready var initial_moon_energy = moon_light.light_energy

@onready var initial_sun_transform = sun_light.global_transform
@onready var initial_moon_transform = moon_light.global_transform

@onready var initial_sun_color = sun_light.light_color
@onready var initial_moon_color = moon_light.light_color

@export var moon_horizon_color := Color("#ff7534")
@export var sun_horizon_color := Color("#8f0025")

const SUN_ORIGIN = 0.32
const MOON_ORIGIN = 0.82


func _ready():
	if Global.is_xr():
		Global.loading_started.connect(self._on_loading_started)
		Global.loading_finished.connect(self._on_loading_finished)


func _on_loading_started():
	print("loading started")
	world_environment.environment.background_energy_multiplier = 0.0
	world_environment.environment.ambient_light_energy = 0.0
	sun_light.light_energy = 0.0
	moon_light.light_energy = 0.0


func _on_loading_finished():
	print("loading finished")
	var tween = get_tree().create_tween().set_parallel(true)
	world_environment.environment.background_energy_multiplier = 0.0
	world_environment.environment.ambient_light_energy = 0.0
	sun_light.light_energy = 0.0
	moon_light.light_energy = 0.0

	tween.tween_property(world_environment, "environment:background_energy_multiplier", 1.0, 1.0)
	tween.tween_property(world_environment, "environment:ambient_light_energy", 1.0, 1.0)
	tween.tween_property(sun_light, "light_energy", initial_sun_energy, 1.0)
	tween.tween_property(moon_light, "light_energy", initial_moon_energy, 1.0)


# Sun and moon light animation
func _process(_delta: float):
	var time = 1.0 + GlobalTime.normalized_time
	var sun_angle = clamp(((time - SUN_ORIGIN) - floor(time - SUN_ORIGIN)) * 2.0, 0.0, 1.0)
	var moon_angle = clamp(((time - MOON_ORIGIN) - floor(time - MOON_ORIGIN)) * 2.0, 0.0, 1.0)

	var sun_t = smoothstep(0.0, .2, sun_angle) * smoothstep(1.0, .8, sun_angle)
	sun_light.visible = !(sun_angle >= .999 || sun_angle <= .001)
	sun_light.light_energy = lerp(0.0, initial_sun_energy, sun_t)
	sun_light.global_transform = (
		initial_sun_transform
		. rotated(Vector3(1.0, 0.0, 0.0), PI * .49)
		. interpolate_with(
			initial_sun_transform.rotated(Vector3(1.0, 0.0, 0.0), -PI * .49), sun_angle
		)
	)
	sun_light.light_color = lerp(sun_horizon_color, initial_sun_color, sun_t)

	var moon_t = smoothstep(0.0, .2, moon_angle) * smoothstep(1.0, .8, moon_angle)
	moon_light.visible = !(moon_angle >= 1.0 || moon_angle <= 0.0)
	moon_light.light_energy = lerp(
		0.0, initial_moon_energy, smoothstep(0.0, .1, moon_angle) * smoothstep(1.0, .9, moon_angle)
	)
	moon_light.global_transform = (
		initial_moon_transform
		. rotated(Vector3(1.0, 0.0, 0.0), PI * .49)
		. interpolate_with(
			initial_moon_transform.rotated(Vector3(1.0, 0.0, 0.0), -PI * .49), moon_angle
		)
	)
	moon_light.light_color = lerp(moon_horizon_color, initial_moon_color, moon_t)
