class_name SkyBase
extends Node

const SUN_ORIGIN = 0.32
const MOON_ORIGIN = 0.82

@export var moon_horizon_color := Color("#ff7534")
@export var sun_horizon_color := Color("#8f0025")

var last_time := 0.0

@onready var world_environment: WorldEnvironment = $WorldEnvironment
@onready var sun_light: DirectionalLight3D = $SunLight
@onready var moon_light: DirectionalLight3D = $MoonLight

@onready var initial_sun_energy = sun_light.light_energy
@onready var initial_moon_energy = moon_light.light_energy

@onready var initial_sun_transform = sun_light.global_transform
@onready var initial_moon_transform = moon_light.global_transform

@onready var initial_sun_color = sun_light.light_color
@onready var initial_moon_color = moon_light.light_color


func _ready():
	if Global.is_xr():
		Global.loading_started.connect(self._on_loading_started)
		Global.loading_finished.connect(self._on_loading_finished)

func on_scene_runner_child_entered_tree(node: Node3D):
	node.hide()
	prints("Hiding:", node.name)

func _on_loading_started():
	print("loading started")
	world_environment.environment.background_energy_multiplier = 0.0
	world_environment.environment.ambient_light_energy = 0.0
	sun_light.light_energy = 0.0
	moon_light.light_energy = 0.0
	
	var scene_runner = Global.get_scene_runner()
	scene_runner.child_entered_tree.connect(self.on_scene_runner_child_entered_tree)
	for child in scene_runner.get_children():
		child.hide()


func _on_loading_finished():
	var scene_runner = Global.get_scene_runner()
	scene_runner.child_entered_tree.disconnect(self.on_scene_runner_child_entered_tree)
	for child in scene_runner.get_children():
		child.show()
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
func setup_light(
	normalized_time: float,
	origin: float,
	light: DirectionalLight3D,
	initial_energy: float,
	horizon_color: Color,
	initial_color: Color,
	initial_transform: Transform3D
):
	var time = 1.0 + normalized_time
	var angle = clamp(((time - origin) - floor(time - origin)) * 2.0, 0.0, 1.0)
	var t = smoothstep(0.0, .2, angle) * smoothstep(1.0, .8, angle)
	light.visible = !(angle >= .999 || angle <= .001)
	light.light_energy = lerp(0.0, initial_energy, t)
	light.global_transform = (
		initial_transform
		. rotated(Vector3(1.0, 0.0, 0.0), PI * .49)
		. interpolate_with(initial_transform.rotated(Vector3(1.0, 0.0, 0.0), -PI * .49), angle)
	)
	light.light_color = lerp(horizon_color, initial_color, t)


func _process(_delta: float):
	var skybox_time = Global.skybox_time.get_normalized_time()
	if last_time == skybox_time:
		return
	last_time = skybox_time
	setup_light(
		skybox_time,
		SUN_ORIGIN,
		sun_light,
		initial_sun_energy,
		sun_horizon_color,
		initial_sun_color,
		initial_sun_transform
	)
	setup_light(
		skybox_time,
		MOON_ORIGIN,
		moon_light,
		initial_moon_energy,
		moon_horizon_color,
		initial_moon_color,
		initial_moon_transform
	)
