class_name SkyBase
extends Node

# Sparse atmosphere keyframes (must match BAKE_HOURS in the snapshot tool).
# Spaced denser around sunrise/sunset where atmosphere changes fastest.
const BAKE_HOURS: Array[float] = [0.0, 4.0, 6.0, 8.0, 12.0, 16.0, 18.0, 20.0]
const BAKE_FACE_ORDER: Array[String] = ["px", "nx", "py", "ny", "pz", "nz"]

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
@export var debug_cycle_seconds: float = 10.0

var _moon_smooth_dir := Vector3(0.0, 1.0, 0.0)
var _baked_cubemaps: Array[Cubemap] = []
var _clouds_cubemap: Cubemap

@onready var world_environment: WorldEnvironment = $WorldEnvironment
@onready var main_light: DirectionalLight3D = $SkyLights/MainLight
@onready var anim_player: AnimationPlayer = $SkyLights/AnimationPlayer
@onready var initial_sun_energy = main_light.light_energy
@onready var sky_material = world_environment.environment.sky.sky_material


func _ready():
	# Set up animation: play, sample at position, stop
	anim_player.play("light_cycle")
	anim_player.pause()

	_build_baked_cubemaps()
	_build_clouds_cubemap()

	if Global.is_xr():
		Global.loading_started.connect(self._on_loading_started)
		Global.loading_finished.connect(self._on_loading_finished)


# Slices the 4×3 cubemap-cross PNG (horizon_clouds.png) into 6 face Images and assembles
# a Cubemap. Sampled with samplerCube + EYEDIR — no equirect distortion at the poles.
# Cross layout (col, row), source = 8192×6144, face = 2048×2048:
#       [    ] [ +Y ] [    ] [    ]
#       [ -X ] [ +Z ] [ +X ] [ -Z ]
#       [    ] [ -Y ] [    ] [    ]
func _build_clouds_cubemap() -> void:
	var src_path := ProjectSettings.globalize_path("res://assets/environment/horizon_clouds.png")
	var src := Image.load_from_file(src_path)
	if src == null:
		push_warning("Missing clouds source: %s" % src_path)
		return

	var face_size: int = src.get_width() / 4
	var face_positions := {
		# Iteration order matches Godot Cubemap layer order: +X, -X, +Y, -Y, +Z, -Z
		"px": Vector2i(2, 1),
		"nx": Vector2i(0, 1),
		"py": Vector2i(1, 0),
		"ny": Vector2i(1, 2),
		"pz": Vector2i(1, 1),
		"nz": Vector2i(3, 1),
	}
	var images: Array[Image] = []
	for face in ["px", "nx", "py", "ny", "pz", "nz"]:
		var pos: Vector2i = face_positions[face]
		var face_img := Image.create(face_size, face_size, false, src.get_format())
		face_img.blit_rect(
			src, Rect2i(pos.x * face_size, pos.y * face_size, face_size, face_size), Vector2i.ZERO
		)
		images.append(face_img)

	var cubemap := Cubemap.new()
	var err := cubemap.create_from_images(images)
	if err != OK:
		push_warning("clouds Cubemap.create_from_images failed (err=%d)" % err)
		return
	_clouds_cubemap = cubemap
	if sky_material:
		sky_material.set_shader_parameter("clouds_cubemap", _clouds_cubemap)


# Loads the 8 sparse Rayleigh/Mie keyframes baked by the snapshot tool and assembles
# them into Cubemap resources. Runtime samples 2 adjacent cubemaps + lerps (Phase E3).
# Uses Image.load_from_file to bypass Godot's import pipeline — VRAM compression (BPTC)
# quantizes each face independently and produces visible seams at cubemap face edges.
func _build_baked_cubemaps() -> void:
	_baked_cubemaps.clear()
	for hour in BAKE_HOURS:
		var hour_int: int = int(hour)
		var images: Array[Image] = []
		var ok := true
		for face in BAKE_FACE_ORDER:
			var rel_path := "res://assets/environment/sky_baked/atm_%02d_%s.png" % [hour_int, face]
			var abs_path := ProjectSettings.globalize_path(rel_path)
			var img := Image.load_from_file(abs_path)
			if img == null:
				push_warning("Missing baked sky face: %s" % abs_path)
				ok = false
				break
			images.append(img)
		if not ok or images.size() != 6:
			continue
		var cubemap := Cubemap.new()
		var err := cubemap.create_from_images(images)
		if err != OK:
			push_warning("Cubemap.create_from_images failed for hour %d (err=%d)" % [hour_int, err])
			continue
		_baked_cubemaps.append(cubemap)


# Picks the two BAKE_HOURS keyframes that bracket the current time, plus a 0..1 blend
# factor between them. Wraps across midnight (last keyframe → first keyframe).
func _update_baked_cubemaps(skybox_time: float) -> void:
	if _baked_cubemaps.size() < 2 or sky_material == null:
		return

	var hour_f: float = skybox_time * 24.0
	var n: int = BAKE_HOURS.size()
	var lo_idx: int = n - 1  # default: between last and first (wrap segment)
	for i in range(n - 1):
		if hour_f >= BAKE_HOURS[i] and hour_f < BAKE_HOURS[i + 1]:
			lo_idx = i
			break

	var hi_idx: int = (lo_idx + 1) % n
	var lo_hour: float = BAKE_HOURS[lo_idx]
	var hi_hour: float = BAKE_HOURS[hi_idx]
	if hi_hour <= lo_hour:
		hi_hour += 24.0  # wrap segment (e.g. 20 → 24/00)
	var hour_unwrapped: float = hour_f
	if hour_unwrapped < lo_hour:
		hour_unwrapped += 24.0
	var blend: float = clamp((hour_unwrapped - lo_hour) / (hi_hour - lo_hour), 0.0, 1.0)

	sky_material.set_shader_parameter("baked_cubemap_a", _baked_cubemaps[lo_idx])
	sky_material.set_shader_parameter("baked_cubemap_b", _baked_cubemaps[hi_idx])
	sky_material.set_shader_parameter("baked_blend_factor", blend)


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
	# gradients all advance with skybox_time. Without this they freeze at the project default.
	RenderingServer.global_shader_parameter_set("day_night_cycle", skybox_time)

	_update_baked_cubemaps(skybox_time)

	# Energy from light elevation (positive when sun above horizon)
	var light_dir = -main_light.global_transform.basis.z
	var elevation = -light_dir.y
	var energy_factor = smoothstep(-0.05, 0.3, elevation)

	main_light.visible = energy_factor > 0.01
	main_light.light_energy = initial_sun_energy * energy_factor

	# Visual sun direction — driven by the animation.
	var sun_dir = main_light.global_transform.basis.z
	RenderingServer.global_shader_parameter_set("sun_direction", sun_dir)

	# Synthetic atmospheric sun position — cycles below horizon at night so Rayleigh/Mie
	# scattering produces dark sky (the visual sun animation keeps the disc above horizon).
	# At t=0 → straight down; t=0.25 → +X horizon (sunrise); t=0.5 → straight up.
	var atm_sun_dir = Vector3(sin(TAU * skybox_time), -cos(TAU * skybox_time), 0.0)
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
