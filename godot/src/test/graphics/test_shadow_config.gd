## Pins the three directional-shadow cost knobs tuned for A54-class
## Mali-G68 mobile fragment work. If a future scene-level edit reverts
## any of these to the Godot defaults the regression shows up here
## instead of as a silent 3 ms render_gpu_ms regression on bench.
extends Node

const SKY_LIGHTS_PATH := "res://assets/environment/sky_lights.tscn"
const EXPECTED_DIRECTIONAL_SHADOW_SIZE := 256
const EXPECTED_MAX_DISTANCE := 30.0
const EXPECTED_SHADOW_MODE := DirectionalLight3D.SHADOW_PARALLEL_2_SPLITS

var suite_name: String = "graphics::test_shadow_config"
var method_name: String = ""
var errors: Array[String] = []


func run() -> bool:
	errors.clear()
	callv(method_name, [])
	return errors.is_empty()


func test_directional_shadow_size_is_256() -> void:
	var actual: Variant = ProjectSettings.get_setting(
		"rendering/lights_and_shadows/directional_shadow/size"
	)
	if int(actual) != EXPECTED_DIRECTIONAL_SHADOW_SIZE:
		errors.append(
			(
				"expected rendering/lights_and_shadows/directional_shadow/size == %d, got %s"
				% [EXPECTED_DIRECTIONAL_SHADOW_SIZE, str(actual)]
			)
		)


func test_main_light_max_distance_is_30() -> void:
	var light := _load_main_light()
	if light == null:
		errors.append("MainLight not found in %s" % SKY_LIGHTS_PATH)
		return
	if not is_equal_approx(light.directional_shadow_max_distance, EXPECTED_MAX_DISTANCE):
		errors.append(
			(
				"expected directional_shadow_max_distance == %.1f, got %f"
				% [EXPECTED_MAX_DISTANCE, light.directional_shadow_max_distance]
			)
		)


func test_main_light_shadow_mode_is_2_splits() -> void:
	var light := _load_main_light()
	if light == null:
		errors.append("MainLight not found in %s" % SKY_LIGHTS_PATH)
		return
	if light.directional_shadow_mode != EXPECTED_SHADOW_MODE:
		errors.append(
			(
				"expected directional_shadow_mode == SHADOW_PARALLEL_2_SPLITS (%d), got %d"
				% [EXPECTED_SHADOW_MODE, light.directional_shadow_mode]
			)
		)


func _load_main_light() -> DirectionalLight3D:
	var scene: PackedScene = load(SKY_LIGHTS_PATH) as PackedScene
	if scene == null:
		return null
	var root: Node = scene.instantiate()
	var light := root.get_node_or_null("MainLight") as DirectionalLight3D
	return light
