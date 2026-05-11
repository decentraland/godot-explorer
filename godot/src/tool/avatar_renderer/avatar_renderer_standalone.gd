extends Node

const USE_TEST_INPUT = false

# Manual supersampling factor on top of viewport.scaling_3d_scale.
# Effective SSAA = SSAA_FACTOR * scaling_3d_scale (= 2 * 2 = 4x here).
# Internal framebuffer = dest_size * SSAA_FACTOR * scaling_3d_scale, e.g.
# a 256x512 capture renders at 1024x2048 internally.
const SSAA_FACTOR = 2

var logs: Array[String] = []

var profiles_to_process: AvatarRendererHelper.AvatarFile
var current_profile_index: int = 0
var current_avatar: DclAvatarWireFormat

@onready var avatar_preview = %AvatarPreview


func get_params_from_cmd():
	# Only use from the editor
	if USE_TEST_INPUT or Global.cli.use_test_input:
		return [
			AvatarRendererHelper.AvatarFile.from_file_path("res://../tests/avatars-test-input.json")
		]

	var avatar_data = null
	var file_path: String = Global.cli.avatars_file
	if not file_path.is_empty():
		avatar_data = AvatarRendererHelper.AvatarFile.from_file_path(file_path)

	return [avatar_data]


func _ready():
	print("spawning avatar renderer scene")

	var from_params = get_params_from_cmd()
	if from_params[0] == null:
		printerr("param is missing or wrong, try with --avatars [file]")
		get_tree().quit(1)
		return

	profiles_to_process = from_params[0]
	if profiles_to_process.profiles.is_empty():
		printerr("no avatars to process")
		get_tree().quit(2)
		return

	# Disable some functions
	Global.scene_runner.set_pause(true)

	Global.realm.content_base_url = profiles_to_process.base_url

	self.start.call_deferred()


func start():
	async_update_avatar(0)

	# Visual enhance — off-screen capture, no realtime perf budget, so
	# stack every AA + quality lever Godot has.
	#   msaa_3d = MSAA_8X: project's defined max for anti_aliasing
	#     (config_data.gd: 0=Off, 1=x2, 2=x4, 3=x8). Smooths geometry
	#     edges (multi-sampled per fragment).
	#   scaling_3d_scale = 2.0: hard-clamped to [0.1, 2.0] in
	#     Viewport::set_scaling_3d_scale. Adds 2x SSAA on top of the
	#     manual SSAA below (so effective SSAA = 2 * SSAA_FACTOR).
	#   screen_space_aa = FXAA: cheap final-pass post AA on top of MSAA
	#     + SSAA. Catches sub-pixel jaggies the upstream passes missed
	#     (especially around alpha cutouts).
	#   mesh_lod_threshold = 0.0: forces every mesh to its highest LOD
	#     regardless of camera distance.
	#   use_debanding: smooths gradient banding in the LDR output.
	var viewport: SubViewport = avatar_preview.subviewport
	viewport.msaa_3d = Viewport.MSAA_8X
	viewport.scaling_3d_scale = 2.0
	viewport.screen_space_aa = Viewport.SCREEN_SPACE_AA_FXAA
	viewport.mesh_lod_threshold = 0.0
	viewport.use_debanding = true
	RenderingServer.screen_space_roughness_limiter_set_active(true, 4.0, 1.0)

	# Brightness compensation for the toon shader (EMISSION = ALBEDO * 0.4
	# floor). avatar_preview.tscn ships with `adjustment_brightness = 1.4`
	# for the in-game UI, but Environment.adjustment_* is silently dropped
	# in the GLES3 Compatibility renderer (Godot issue #92853). Replace it
	# with `tonemap_exposure` here, scoped to the snapshot tool so the
	# backpack/lobby/profile UIs keep their existing tuning.
	#
	# Compat goes through an LDR sRGB framebuffer (RGBA8) while Mobile/Vulkan
	# uses HDR linear, so the same exposure value lands ~1.7x dimmer in
	# Compat after the gamma round-trip. Apply a 2.8x boost on top when
	# running GLES3 (no rendering device) so the PNG matches Mobile/prod.
	var shared_env: Environment = avatar_preview.world_environment.environment
	if shared_env != null:
		var env: Environment = shared_env.duplicate()
		env.adjustment_enabled = false
		env.tonemap_exposure = 1.4
		if RenderingServer.get_rendering_device() == null:
			env.tonemap_exposure *= 2.8
		avatar_preview.world_environment.environment = env


func flush_logs():
	for log_item in logs:
		print(log_item)
	logs.clear()


func async_update_avatar(index: int):
	var profile: AvatarRendererHelper.AvatarRendererSpecs = profiles_to_process.profiles[index]

	current_avatar = profile.avatar
	current_profile_index = index

	if not profile.entity.is_empty():
		prints("processing payload entity", profile.entity)
	else:
		prints("processing payload", current_profile_index)

	await get_tree().process_frame

	await avatar_preview.avatar.async_update_avatar(current_avatar, "")

	await _async_on_avatar_avatar_loaded()


func ensure_ends_with(path: String, ends: String) -> String:
	if not path.ends_with(ends):
		return path + ends

	return path


func ensure_base_dir_exists(path: String) -> void:
	var dir = path.get_base_dir()
	if not DirAccess.dir_exists_absolute(dir):
		DirAccess.make_dir_recursive_absolute(dir)


func _async_on_avatar_avatar_loaded():
	var profile := profiles_to_process.profiles[current_profile_index]
	RenderingServer.set_default_clear_color(Color(0, 0, 0, 0))

	# Toon shader is now used directly. Brightness compensation moved to
	# Environment.tonemap_exposure (renderer-agnostic, works in
	# Compatibility unlike adjustment_brightness). See avatar_preview.tscn.
	await get_tree().process_frame

	var dest_path := ensure_ends_with(profile.dest_path, ".png")
	ensure_base_dir_exists(dest_path)

	var body_image = await avatar_preview.async_get_viewport_image(
		false, Vector2i(profile.width, profile.height), 2.5, SSAA_FACTOR
	)
	body_image.save_png(dest_path)
	logs.push_back("🟢 " + dest_path)

	if not profile.face_dest_path.is_empty():
		var face_dest_path := ensure_ends_with(profile.face_dest_path, ".png")
		ensure_base_dir_exists(face_dest_path)

		var face_image = await avatar_preview.async_get_viewport_image(
			true, Vector2i(profile.face_width, profile.face_height), profile.face_zoom, SSAA_FACTOR
		)
		face_image.save_png(face_dest_path)
		logs.push_back("🟢 " + face_dest_path)

	flush_logs()
	await get_tree().process_frame

	if current_profile_index >= profiles_to_process.profiles.size() - 1:
		Global.testing_tools.exit_gracefully(0)
	else:
		async_update_avatar.call_deferred(current_profile_index + 1)
