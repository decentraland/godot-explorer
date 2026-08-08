extends Node

# Standalone lighting snapshot renderer — visual regression harness for the
# day/night cycle (issue #2516).
#
# Renders a small synthetic diorama (ground + boxes + capsule "avatar") under
# sky_high at fixed times of day and saves one PNG per capture to
# res://output/. The xtask `test-tools` target then moves those into
# tests/snapshots/lighting/comparison/ and pixel-diffs them against the
# committed baselines with a 0.95 similarity threshold.
#
# The capture happens in a fixed-size SubViewport (like the avatar renderer)
# so PNG dimensions don't depend on the host window size / DPI scale.
#
# Run:
#   godot4_bin --path godot --lighting-renderer
#   # or, full pipeline with comparison:
#   cargo run -- test-tools

const SKY_HIGH = preload("res://assets/environment/sky_high/sky_high.tscn")

const OUTPUT_DIR := "res://output/"
const VIEWPORT_SIZE := Vector2i(1280, 720)

# Frames to wait after changing the time so SkyBase._process, the shadow map
# and the sky shader settle before capturing.
const SETTLE_FRAMES := 10

# (name, seconds-of-day): the four acceptance-criteria times of issue #2516
# plus a golden-hour capture, where the sun grazed the horizon and ground
# shadows used to disappear.
const CAPTURES: Array = [
	["night_00h", 0],
	["morning_08h", 8 * 3600],
	["midday_12h", 12 * 3600],
	["afternoon_17h", 17 * 3600],
	["golden_18h", 18 * 3600],
]

var _sky: Node = null


# gdlint:ignore = async-function-name
func _ready() -> void:
	# Mirror the High graphics profile (what issue #2516 was reported on) so
	# captures show the same shadows as the real client.
	RenderingServer.directional_shadow_atlas_set_size(2048, false)
	RenderingServer.directional_soft_shadow_filter_set_quality(
		RenderingServer.SHADOW_QUALITY_SOFT_MEDIUM
	)
	var viewport: SubViewport = $SubViewport
	viewport.size = VIEWPORT_SIZE
	_build_diorama(viewport)
	_build_camera(viewport)
	_sky = SKY_HIGH.instantiate()
	viewport.add_child(_sky)
	# Mirror the High shadow profile for the moon light too (absent on the
	# pre-fix sky, which has no MoonLight node).
	var moon_light := _sky.get_node_or_null("SkyLights/MoonLight") as DirectionalLight3D
	if moon_light != null:
		moon_light.shadow_enabled = true
	await _run_captures()


func _debug_lights(label: String) -> void:
	var main_light: DirectionalLight3D = _sky.main_light
	var moon_light := _sky.get_node_or_null("SkyLights/MoonLight") as DirectionalLight3D
	var env: Environment = _sky.world_environment.environment
	prints(
		"  [debug]",
		label,
		"| sun visible:",
		main_light.visible,
		"energy:",
		main_light.light_energy,
		"to-sun.y:",
		main_light.global_transform.basis.z.y,
		"| moon visible:",
		moon_light.visible if moon_light else null,
		"energy:",
		moon_light.light_energy if moon_light else null,
		"| ambient:",
		env.ambient_light_color * env.ambient_light_energy
	)


# gdlint:ignore = async-function-name
func _run_captures() -> void:
	var config := Global.get_config()
	config.dynamic_skybox = false

	if not DirAccess.dir_exists_absolute(OUTPUT_DIR):
		DirAccess.make_dir_recursive_absolute(OUTPUT_DIR)

	for capture in CAPTURES:
		config.skybox_time = capture[1]
		for i in SETTLE_FRAMES:
			await get_tree().process_frame
		await RenderingServer.frame_post_draw
		var image: Image = $SubViewport.get_texture().get_image()
		var path: String = OUTPUT_DIR + "lighting_" + capture[0] + ".png"
		image.save_png(path)
		prints("lighting-renderer: saved", path)
		_debug_lights(capture[0])

	print("lighting-renderer: done")
	get_tree().quit(0)


func _build_camera(viewport: SubViewport) -> void:
	var camera := Camera3D.new()
	viewport.add_child(camera)
	camera.position = Vector3(7.0, 4.0, 7.0)
	camera.look_at_from_position(camera.position, Vector3(0.0, 0.8, 0.0), Vector3.UP)


func _build_diorama(viewport: SubViewport) -> void:
	# Neutral-gray ground, like plaza paving: makes color-grade shifts and
	# ground shadows easy to read in the diff.
	var ground := MeshInstance3D.new()
	var ground_mesh := PlaneMesh.new()
	ground_mesh.size = Vector2(24.0, 24.0)
	ground.mesh = ground_mesh
	ground.material_override = _make_material(Color(0.62, 0.62, 0.62))
	viewport.add_child(ground)

	# White box: reads the key-light color directly.
	var box := MeshInstance3D.new()
	var box_mesh := BoxMesh.new()
	box_mesh.size = Vector3(1.5, 1.5, 1.5)
	box.mesh = box_mesh
	box.position = Vector3(-2.0, 0.75, -1.0)
	box.material_override = _make_material(Color(0.9, 0.9, 0.9))
	viewport.add_child(box)

	# Tall pillar: casts a long, easy-to-diff ground shadow at low sun angles.
	var pillar := MeshInstance3D.new()
	var pillar_mesh := BoxMesh.new()
	pillar_mesh.size = Vector3(0.6, 4.0, 0.6)
	pillar.mesh = pillar_mesh
	pillar.position = Vector3(2.2, 2.0, -1.5)
	pillar.material_override = _make_material(Color(0.8, 0.8, 0.85))
	viewport.add_child(pillar)

	# Capsule "avatar": stands in for character legibility at night.
	var avatar := MeshInstance3D.new()
	var capsule_mesh := CapsuleMesh.new()
	capsule_mesh.radius = 0.35
	capsule_mesh.height = 1.8
	avatar.mesh = capsule_mesh
	avatar.position = Vector3(0.3, 0.9, 1.2)
	avatar.material_override = _make_material(Color(0.75, 0.55, 0.45))
	viewport.add_child(avatar)


func _make_material(albedo: Color) -> StandardMaterial3D:
	var material := StandardMaterial3D.new()
	material.albedo_color = albedo
	material.roughness = 0.9
	return material
