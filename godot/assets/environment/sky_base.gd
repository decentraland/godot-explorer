class_name SkyBase
extends Node

var last_time := 0.0

@onready var world_environment: WorldEnvironment = $WorldEnvironment
@onready var sky_material = world_environment.environment.sky.sky_material
@onready
var day_fog_color = sky_material.get_shader_parameter("clouds_gradient_day").gradient.colors[0]
@onready
var night_fog_color = sky_material.get_shader_parameter("clouds_gradient_night").gradient.colors[0]


func _ready():
	if Global.is_xr():
		Global.loading_started.connect(self._on_loading_started)
		Global.loading_finished.connect(self._on_loading_finished)
	if OS.get_name() == "iOS":
		world_environment.environment.glow_enabled = false


func on_scene_runner_child_entered_tree(node: Node3D):
	node.hide()
	prints("Hiding:", node.name)


func _on_loading_started():
	world_environment.environment.background_energy_multiplier = 0.0
	world_environment.environment.ambient_light_energy = 0.0

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

	tween.tween_property(world_environment, "environment:background_energy_multiplier", 1.0, 1.0)
	tween.tween_property(world_environment, "environment:ambient_light_energy", 1.0, 1.0)


func _process(_delta: float) -> void:
	var cycle = Global.skybox_time.get_normalized_time() + .43
	cycle -= floor(cycle)
	var blend = sin(cycle * PI)
	world_environment.environment.fog_light_color = day_fog_color.lerp(night_fog_color, blend)
