extends Node

const USE_TEST_INPUT = false

## Per-item watchdog: a hung content fetch or load marks the item as errored
## and the batch continues (the avatar renderer can stall forever on one bad
## asset; this tool must not — the caller relies on the report).
const ITEM_TIMEOUT_SECONDS = 60.0

## Exclude the outline layer (20), same trick as the scene renderer.
const CAMERA_CULL_MASK = 524287

const NEUTRAL_COLOR = {"color": {"r": 0.35, "g": 0.35, "b": 0.35}}

var input: AssetRendererInputHelper.AssetInputFile
var results: Array[Dictionary] = []

# Watchdog bookkeeping: _generation guards a timed-out item coroutine from
# committing into a later item's slot (the zombie keeps running — see
# _async_process_item_guarded).
var _generation := 0
var _pending_result = null

@onready var avatar_preview = %AvatarPreview


func get_params_from_cmd():
	if USE_TEST_INPUT or Global.cli.use_test_input:
		return AssetRendererInputHelper.AssetInputFile.from_file_path(
			"res://../tests/asset-renderer-test-input.json"
		)

	var file_path: String = Global.cli.asset_input_file
	if file_path.is_empty():
		return null
	return AssetRendererInputHelper.AssetInputFile.from_file_path(file_path)


func _ready():
	print("spawning asset renderer scene")

	input = get_params_from_cmd()
	if input == null:
		printerr("param is missing or wrong, try with --asset-input-file [file]")
		get_tree().quit(1)
		return

	if input.items.is_empty() and input.invalid_items.is_empty():
		printerr("no assets to process")
		get_tree().quit(2)
		return

	Global.scene_runner.set_pause(true)
	Global.realm.content_base_url = input.base_url

	# The preview's _process lerps camera size/pan toward UI targets; this tool
	# drives the camera directly per shot, so the lerp must never run.
	avatar_preview.set_process(false)
	avatar_preview.fit_avatar = false

	RenderingServer.set_default_clear_color(Color(0, 0, 0, 0))
	_setup_render_quality()

	self._async_start.call_deferred()


func _setup_render_quality():
	# Same off-screen quality levers as the avatar renderer, except
	# scaling_3d_scale: that one is caller-controlled (supersample) and
	# defaults to 1.0 because llvmpipe pays 4x raster cost at 2.0.
	var viewport: SubViewport = avatar_preview.subviewport
	viewport.msaa_3d = Viewport.MSAA_4X
	viewport.scaling_3d_scale = clampf(input.supersample, 0.5, 2.0)
	viewport.screen_space_aa = Viewport.SCREEN_SPACE_AA_FXAA
	viewport.mesh_lod_threshold = 0.0
	viewport.use_debanding = true
	RenderingServer.screen_space_roughness_limiter_set_active(true, 4.0, 1.0)

	# Toon-shader brightness compensation, copied from the avatar renderer:
	# Environment.adjustment_* is silently dropped in the GLES3 Compatibility
	# renderer (Godot issue #92853), so replace it with tonemap_exposure, with
	# the extra 2.8x LDR boost when no rendering device is present.
	var shared_env: Environment = avatar_preview.world_environment.environment
	if shared_env != null:
		var env: Environment = shared_env.duplicate()
		env.adjustment_enabled = false
		env.tonemap_exposure = 1.4
		if RenderingServer.get_rendering_device() == null:
			env.tonemap_exposure *= 2.8
		avatar_preview.world_environment.environment = env


func _async_start():
	var camera: Camera3D = avatar_preview.camera_3d
	camera.top_level = true
	camera.cull_mask = CAMERA_CULL_MASK

	for invalid in input.invalid_items:
		results.push_back(
			{"id": invalid.id, "status": "error", "error": invalid.error, "files": []}
		)

	for item in input.items:
		var result: Dictionary = await _async_process_item_guarded(item)
		results.push_back(result)
		var status_icon = "🟢" if result.status == "ok" else "🔴"
		prints(status_icon, item.id, result.get("error", ""))

	_write_report()

	var all_ok := results.all(func(result): return result.status == "ok")
	Global.testing_tools.exit_gracefully(0 if all_ok else 3)


func _async_process_item_guarded(item: AssetRendererInputHelper.AssetItem) -> Dictionary:
	_generation += 1
	var generation := _generation
	_pending_result = null
	_async_process_item(item, generation)

	var deadline := Time.get_ticks_msec() + int(ITEM_TIMEOUT_SECONDS * 1000.0)
	while _pending_result == null and Time.get_ticks_msec() < deadline:
		await get_tree().process_frame

	if _pending_result == null:
		# The coroutine keeps running detached; the generation check stops it
		# from committing a result later. Its avatar state, if it ever lands,
		# is overwritten by the next item's async_update_avatar.
		return {"id": item.id, "status": "error", "error": "timeout", "files": []}
	var result: Dictionary = _pending_result
	return result


func _commit_result(generation: int, result: Dictionary) -> void:
	if generation == _generation:
		_pending_result = result


func _neutral_avatar_dictionary(item: AssetRendererInputHelper.AssetItem) -> Dictionary:
	var dictionary := {
		"bodyShape": item.body_shape,
		"eyes": NEUTRAL_COLOR,
		"hair": NEUTRAL_COLOR,
		"skin": NEUTRAL_COLOR,
		"wearables": [],
		"emotes": []
	}
	match item.kind:
		"wearable_standalone":
			dictionary["wearables"] = [item.urn]
			# Hides every body part so the only visible pixels are the item's
			dictionary["showOnlyWearables"] = true
		"wearable_on_avatar":
			dictionary["wearables"] = [item.urn]
			dictionary.merge(item.avatar_overrides, true)
	return dictionary


func _async_process_item(item: AssetRendererInputHelper.AssetItem, generation: int) -> void:
	var wire = DclAvatarWireFormat.from_godot_dictionary(_neutral_avatar_dictionary(item))

	# from_godot_dictionary falls back to a DEFAULT avatar when the dictionary
	# fails to parse (json5 unwrap_or_default), which would silently render a
	# generic mannequin as if it were the asset — catch that here. Skipped when
	# the caller supplied its own wearables list, which legitimately may not
	# list the target urn.
	var expects_urn: bool = item.kind != "emote" and not item.avatar_overrides.has("wearables")
	if expects_urn and not item.urn in wire.get_wearables():
		_commit_result(
			generation,
			{
				"id": item.id,
				"status": "error",
				"error": "avatar payload did not parse (check the avatar overrides)",
				"files": []
			}
		)
		return

	await avatar_preview.avatar.async_update_avatar(wire, "")
	if generation != _generation:
		return

	var emote_controller = avatar_preview.avatar.emote_controller
	if item.kind != "emote":
		emote_controller.freeze_on_idle()
	avatar_preview.avatar.rotation.y = 0.0
	await get_tree().process_frame

	if (
		item.kind == "wearable_standalone"
		and _is_fallback_aabb(avatar_preview.compute_current_fit_aabb())
	):
		_commit_result(
			generation,
			{"id": item.id, "status": "error", "error": "wearable has no visible mesh", "files": []}
		)
		return

	var files: Array[String] = []
	for shot in item.shots:
		# A timed-out item keeps running detached; stop before it resizes the
		# shared viewport underneath the item being captured now.
		if generation != _generation:
			return

		if item.kind == "emote":
			var frozen: bool = await emote_controller.async_freeze_on_emote(item.urn, shot.at_time)
			if not frozen:
				_commit_result(
					generation,
					{
						"id": item.id,
						"status": "error",
						"error": "emote could not be posed",
						"files": files
					}
				)
				return
			await get_tree().process_frame

		_place_camera(shot.camera)

		var dest_path := _ensure_ends_with(shot.dest_path, ".png")
		_ensure_base_dir_exists(dest_path)
		# Explicitly typed: awaiting a coroutine yields an untyped value, so `:=`
		# cannot infer here (gdlint accepts it, the Godot compiler does not).
		var image: Image = await avatar_preview.async_capture_current_view(
			Vector2i(shot.width, shot.height), 1
		)
		var save_error: int = image.save_png(dest_path)
		if save_error != OK:
			_commit_result(
				generation,
				{
					"id": item.id,
					"status": "error",
					"error": "could not save %s (error %d)" % [dest_path, save_error],
					"files": files
				}
			)
			return
		files.push_back(dest_path)

	_commit_result(generation, {"id": item.id, "status": "ok", "files": files})


func _is_fallback_aabb(aabb: AABB) -> bool:
	return aabb == AABB(Vector3(-1.0, 0.0, -1.0), Vector3(2.0, 2.0, 2.0))


func _place_camera(shot_camera: AssetRendererInputHelper.ShotCamera) -> void:
	var camera: Camera3D = avatar_preview.camera_3d
	var is_ortho: bool = shot_camera.projection == "ortho"
	camera.projection = (
		Camera3D.PROJECTION_ORTHOGONAL if is_ortho else Camera3D.PROJECTION_PERSPECTIVE
	)
	camera.fov = clampf(shot_camera.fov, 1.0, 179.0)

	if not shot_camera.auto_fit:
		# Explicit camera: scene-renderer semantics (absolute position/target)
		camera.size = maxf(shot_camera.ortho_size, 0.001)
		_look_from(camera, shot_camera.position, shot_camera.target)
		return

	# Fit the posed asset's bounding sphere: conservative (uses the AABB
	# diagonal) so nothing crops regardless of orbit angle.
	var local_aabb: AABB = avatar_preview.compute_current_fit_aabb()
	var avatar_xform: Transform3D = avatar_preview.avatar.global_transform
	var center: Vector3 = avatar_xform * local_aabb.get_center()
	var radius: float = (
		maxf(local_aabb.size.length() * 0.5, 0.05) * maxf(shot_camera.fit_margin, 1.0)
	)

	var yaw := deg_to_rad(shot_camera.orbit_yaw_degrees)
	var pitch := deg_to_rad(clampf(shot_camera.orbit_pitch_degrees, -80.0, 80.0))
	# yaw 0 faces the avatar's front (-Z, matching the profile body camera)
	var direction := Vector3(sin(yaw) * cos(pitch), sin(pitch), -cos(yaw) * cos(pitch))

	if is_ortho:
		camera.size = maxf(radius * 2.0, 0.1)
		_look_from(camera, center + direction * maxf(radius * 4.0, 2.0), center)
	else:
		var distance: float = maxf(radius / sin(deg_to_rad(camera.fov) * 0.5), radius + 0.5)
		_look_from(camera, center + direction * distance, center)


func _look_from(camera: Camera3D, from_position: Vector3, target: Vector3) -> void:
	var up := Vector3.UP
	if up.cross(target - from_position).is_zero_approx():
		up = Vector3.FORWARD
	camera.global_position = from_position
	camera.look_at(target, up)


func _ensure_ends_with(path: String, ends: String) -> String:
	if not path.ends_with(ends):
		return path + ends
	return path


func _ensure_base_dir_exists(path: String) -> void:
	var dir = path.get_base_dir()
	if not DirAccess.dir_exists_absolute(dir):
		DirAccess.make_dir_recursive_absolute(dir)


func _write_report() -> void:
	_ensure_base_dir_exists(input.output_json_path)
	var file = FileAccess.open(input.output_json_path, FileAccess.WRITE)
	if file == null:
		printerr("could not write report to ", input.output_json_path)
		return
	file.store_string(JSON.stringify({"version": 1, "results": results}, "  "))
	file.close()
	print("report written to ", input.output_json_path)
