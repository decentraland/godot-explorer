extends Node

const USE_TEST_INPUT = false

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
	# Restore pre-PR-#1438 PBR lighting for snapshot generation only.
	#
	# Background: PR #1438 ("feat: migrate avatar materials to DCL_Toon
	# shader") swapped every BaseMaterial3D on avatar meshes for a
	# ShaderMaterial backed by dcl_toon.gdshader. The toon shader pins
	# EMISSION = ALBEDO * 0.4, ignores LIGHT_COLOR, and disables ambient
	# — which produces a flat / washed-out look in headless captures
	# (visible vs the prod-style PBR rendering used before that PR).
	#
	# We don't touch the toon shader. Instead, after the avatar loads
	# we walk every MeshInstance3D and put a StandardMaterial3D
	# reconstructed from the toon ShaderMaterial's parameters in the
	# surface override slot. The override beats the mesh's surface
	# material, so this avatar instance renders with normal PBR while
	# the rest of the app keeps the toon shader.
	async_update_avatar(0)

	# Visual enhance
	var viewport: Viewport = avatar_preview.subviewport.get_viewport()
	viewport.use_debanding = true
	viewport.scaling_3d_scale = 2.0
	RenderingServer.screen_space_roughness_limiter_set_active(true, 4.0, 1.0)


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

	# Replace the toon ShaderMaterials applied by avatar.gd with PBR
	# StandardMaterial3D overrides for the snapshot tool only. See the
	# block-comment in start() for context.
	_apply_pbr_overrides(avatar_preview.avatar)
	# Give Godot a couple of frames to register material changes.
	await get_tree().process_frame
	await get_tree().process_frame

	var dest_path := ensure_ends_with(profile.dest_path, ".png")
	ensure_base_dir_exists(dest_path)

	var body_image = await avatar_preview.async_get_viewport_image(
		false, Vector2i(profile.width, profile.height)
	)
	body_image.save_png(dest_path)
	logs.push_back("🟢 " + dest_path)

	if not profile.face_dest_path.is_empty():
		var face_dest_path := ensure_ends_with(profile.face_dest_path, ".png")
		ensure_base_dir_exists(face_dest_path)

		var face_image = await avatar_preview.async_get_viewport_image(
			true, Vector2i(profile.face_width, profile.face_height), profile.face_zoom
		)
		face_image.save_png(face_dest_path)
		logs.push_back("🟢 " + face_dest_path)

	flush_logs()
	await get_tree().process_frame

	if current_profile_index >= profiles_to_process.profiles.size() - 1:
		Global.testing_tools.exit_gracefully(0)
	else:
		async_update_avatar.call_deferred(current_profile_index + 1)


# Walk the avatar tree and replace each toon ShaderMaterial with an
# equivalent StandardMaterial3D placed in the surface override slot.
# Used only by the snapshot tool — does not affect the in-game avatar.
func _apply_pbr_overrides(root: Node) -> void:
	if root == null:
		return
	if root is MeshInstance3D and root.mesh != null:
		for surface_idx in range(root.mesh.get_surface_count()):
			var mat = root.mesh.surface_get_material(surface_idx)
			if mat is ShaderMaterial:
				var pbr = _toon_to_pbr(mat)
				if pbr != null:
					root.set_surface_override_material(surface_idx, pbr)
	for child in root.get_children():
		_apply_pbr_overrides(child)


func _toon_to_pbr(toon: ShaderMaterial) -> StandardMaterial3D:
	var shader := toon.shader
	if shader == null:
		return null
	# Use the shader's resource path to recover transparency / cull mode.
	var path: String = shader.resource_path
	if not path.contains("dcl_toon"):
		# Not one of our toon variants — leave it alone.
		return null

	var pbr := StandardMaterial3D.new()

	var albedo_color = toon.get_shader_parameter("albedo_color")
	if albedo_color is Color:
		pbr.albedo_color = albedo_color
	var albedo_tex = toon.get_shader_parameter("albedo_texture")
	if albedo_tex is Texture2D:
		pbr.albedo_texture = albedo_tex

	var emission_color = toon.get_shader_parameter("emission_color")
	if emission_color is Color and (emission_color.r + emission_color.g + emission_color.b) > 0.0:
		pbr.emission_enabled = true
		pbr.emission = emission_color
		var emission_tex = toon.get_shader_parameter("emission_texture")
		if emission_tex is Texture2D:
			pbr.emission_texture = emission_tex

	if path.contains("alpha_clip"):
		pbr.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA_SCISSOR
		var thresh = toon.get_shader_parameter("alpha_scissor_threshold")
		if thresh is float:
			pbr.alpha_scissor_threshold = thresh
	elif path.contains("alpha_blend"):
		pbr.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA

	if path.contains("double"):
		pbr.cull_mode = BaseMaterial3D.CULL_DISABLED

	pbr.roughness = 1.0
	pbr.metallic = 0.0
	return pbr
