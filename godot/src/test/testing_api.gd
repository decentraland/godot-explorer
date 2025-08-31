class_name TestingTools
extends DclTestingTools

const DEFAULT_TIMEOUT_REALM_SECONDS = 15.0
const DEFAULT_TIMEOUT_TEST_SECONDS = 15.0


class SceneTestItem:
	extends RefCounted
	const INVALID_PARCEL_POSITION = Vector2i.MAX
	const INVALID_SCENE_URN = "invalid"

	var parcel_position: Vector2i = INVALID_PARCEL_POSITION
	var scene_urn: String = INVALID_SCENE_URN
	var timeout_ms: int
	var timeout_set: int
	var test_finished: bool = false
	var test_result: Dictionary = {}

	var already_telep = false

	func _init(parcel_pos: Vector2i, _scene_urn: String) -> void:
		self.scene_urn = _scene_urn
		self.parcel_position = parcel_pos
		reset_timeout()

	func timeout_duration_secs() -> float:
		return float(timeout_ms - timeout_set) / 1000.0

	func reset_timeout() -> void:
		timeout_set = Time.get_ticks_msec()
		timeout_ms = Time.get_ticks_msec() + int(1000.0 * DEFAULT_TIMEOUT_TEST_SECONDS)

	func timeout() -> bool:
		return Time.get_ticks_msec() > timeout_ms


var scene_tests: Array[SceneTestItem] = []
var realm_change_emited: bool = false

var test_camera_node: DclCamera3D
var test_player_body_node: Node3D

var snapshot_folder: String = ""
var snapshot_comparison_folder: String = ""


func _ready():
	self.process_mode = PROCESS_MODE_DISABLED
	start.call_deferred()


func start():
	var scene_test_index := -1
	if Global.cli.scene_test_mode:
		scene_test_index = 0

	if Global.FORCE_TEST:
		scene_test_index = 0
		Global.testing_scene_mode = true

	if scene_test_index == -1:
		self.process_mode = PROCESS_MODE_DISABLED
		return

	if not Global.cli.snapshot_folder.is_empty():
		snapshot_folder = Global.cli.snapshot_folder
	else:
		if OS.has_feature("editor"):
			snapshot_folder = ProjectSettings.globalize_path("res://../tests/snapshots")
		else:
			snapshot_folder = OS.get_user_data_dir() + "/snapshot"

	if not snapshot_folder.ends_with("/"):
		snapshot_folder += "/"

	snapshot_comparison_folder = snapshot_folder + "comparison/"

	if not DirAccess.dir_exists_absolute(snapshot_folder):
		DirAccess.make_dir_recursive_absolute(snapshot_folder)

	if not DirAccess.dir_exists_absolute(snapshot_comparison_folder):
		DirAccess.make_dir_recursive_absolute(snapshot_comparison_folder)

	prints('snapshot_folder="' + snapshot_folder + '"')
	prints('snapshot_comparison_folder="' + snapshot_comparison_folder + '"')

	var parcels_str: String = Global.FORCE_TEST_ARG
	if not Global.FORCE_TEST:
		# Get the scene test argument value
		var scene_test_arg = Global.cli.get_arg("--scene-test")
		if not scene_test_arg.is_empty():
			parcels_str = scene_test_arg.replace("'", '"')
		else:
			# Fallback to getting from args array for backward compatibility
			var args := OS.get_cmdline_args()
			var idx := args.find("--scene-test")
			if idx != -1 and args.size() > idx + 1:
				parcels_str = args[idx + 1].replace("'", '"')

	prints("parcels_str=" + str(parcels_str))

	var parcels = JSON.parse_string(parcels_str)
	for pos_array in parcels:
		if not pos_array is Array:
			continue

		if pos_array.size() == 2:
			var parcel_pos: Vector2i = Vector2i(int(pos_array[0]), int(pos_array[1]))
			scene_tests.push_back(SceneTestItem.new(parcel_pos, ""))
		else:
			printerr("Scene to test '" + pos_array + "' not supported for now.")

	if scene_tests.is_empty():
		printerr(
			"Couldn't get any scene to test in the scene-test mode. Please try --scene-test [[52,-52]]"
		)
		get_tree().quit(1)
		return

	print("Testing scenes: " + str(scene_tests.size()))

	Global.realm.realm_changed.connect(self.on_realm_changed)
	get_tree().create_timer(DEFAULT_TIMEOUT_REALM_SECONDS).timeout.connect(
		self.on_realm_change_timeout
	)

	Global.set_scene_log_enabled(true)


func on_realm_changed():
	realm_change_emited = true
	self.process_mode = Node.PROCESS_MODE_ALWAYS

	test_camera_node = Global.scene_runner.camera_node
	test_player_body_node = Global.scene_runner.player_body_node
	var test_player_avatar_node = Global.scene_runner.player_avatar_node

	Global.scene_runner.set_camera_and_player_node(
		test_camera_node,
		test_player_avatar_node,
		test_player_body_node,
		self._on_scene_console_message
	)
	Global.scene_fetcher.set_scene_radius(0)

	reset_all_timeout()


func _on_scene_console_message(scene_id: int, level: int, timestamp: float, text: String) -> void:
	prints("SCENE_LOG", scene_id, level, timestamp, text)


func on_realm_change_timeout():
	if not realm_change_emited:
		printerr(str(DEFAULT_TIMEOUT_REALM_SECONDS) + " seconds realm changed timeout")
		get_tree().quit(1)
		return

	realm_change_emited = true


func async_take_and_compare_snapshot(
	scene_id: int,
	src_stored_snapshot: String,
	camera_position: Vector3,
	camera_target: Vector3,
	screenshot_size: Vector2,
	method: Dictionary,
	dcl_rpc_sender: DclRpcSenderTakeAndCompareSnapshotResponse
):
	prints(
		"async_take_and_compare_snapshot",
		scene_id,
		src_stored_snapshot,
		camera_position,
		camera_target,
		screenshot_size,
		method,
		dcl_rpc_sender
	)

	var pending_promises := Global.content_provider.get_pending_promises()
	if not pending_promises.is_empty():
		await PromiseUtils.async_all(Global.content_provider.get_pending_promises())

	RenderingServer.set_default_clear_color(Color(0, 0, 0, 0))
	var viewport = get_viewport()
	var previous_camera = viewport.get_camera_3d()

	var test_camera_3d = Camera3D.new()
	add_child(test_camera_3d)
	test_camera_3d.make_current()

	var previous_viewport_size = viewport.size

	viewport.size = screenshot_size
	test_camera_3d.global_position = camera_position
	test_camera_3d.look_at(camera_target)

	var explorer = Global.get_explorer()
	explorer.set_visible_ui(false)

	# Freeze avatars animation and hide them
	for avatar in Global.avatars.get_children():
		if avatar is Avatar:
			avatar.hide()
			avatar.emote_controller.freeze_on_idle()

	Global.scene_runner.player_avatar_node.emote_controller.freeze_on_idle()
	Global.scene_runner.player_avatar_node.hide()

	await get_tree().process_frame
	await get_tree().process_frame
	await get_tree().process_frame

	var viewport_img := viewport.get_texture().get_image()

	# Test: Uncomment this to see how the snapshot would look like
	# await get_tree().create_timer(10.0).timeout

	get_node("/root/explorer").set_visible_ui(true)
	# TODO: should unfreeze avatars?

	viewport.size = previous_viewport_size
	previous_camera.make_current()
	remove_child(test_camera_3d)
	test_camera_3d.queue_free()

	var base_path := (
		src_stored_snapshot.replace(" ", "_").replace("/", "_").replace("\\", "_").to_lower()
	)
	if not base_path.ends_with(".png"):
		base_path += ".png"

	var current_snapshot_path := snapshot_comparison_folder + base_path
	var existing_snapshot_path := snapshot_folder + base_path
	var existing_snapshot: Image = null

	if FileAccess.file_exists(existing_snapshot_path):
		existing_snapshot = Image.load_from_file(existing_snapshot_path)

	viewport_img.save_png(current_snapshot_path)

	var result = {"stored_snapshot_found": existing_snapshot != null}
	if existing_snapshot != null:
		compare(
			method,
			existing_snapshot,
			viewport_img,
			result,
			current_snapshot_path.replace(".png", ".diff.png")
		)

	dcl_rpc_sender.send(result)


func compare(
	method: Dictionary, image_a: Image, image_b: Image, result: Dictionary, diff_dest_path: String
) -> void:
	if method.get("grey_pixel_diff") != null:
		var similarity = self.compute_image_similarity(image_a, image_b, diff_dest_path)
		result["grey_pixel_diff"] = {"similarity": similarity}


func reset_all_timeout():
	for scene in scene_tests:
		scene.reset_timeout()


func dump_test_result_and_get_ok() -> bool:
	var ok: bool = true
	var fail: int = 0
	for scene in scene_tests:
		if scene.test_result.is_empty():
			ok = false
			prints("🔴 test result is empty in the scene " + str(scene.parcel_position))
			continue

		prints(scene.test_result.text)
		prints(scene.test_result.text_detail_failed)
		fail += scene.test_result.fail

	if fail > 0 or not ok:
		prints("Some tests fail or some scenes couldn't be tested")
		return false

	prints("All test of all scene passed")
	return false


func _process(_delta):
	for scene in scene_tests:
		if not scene.test_finished:
			if scene.parcel_position != SceneTestItem.INVALID_PARCEL_POSITION:
				var scene_id: int = Global.scene_fetcher.get_parcel_scene_id(
					scene.parcel_position.x, scene.parcel_position.y
				)
				if scene_id == -1:
					if not scene.already_telep:
						scene.already_telep = true
						scene.reset_timeout()
						test_player_body_node.global_position = Vector3(
							scene.parcel_position.x * 16.0 + 8.0,
							1.0,
							-scene.parcel_position.y * 16.0 - 8.0
						)
					elif scene.timeout():
						printerr(
							(
								"Scene test timeout after "
								+ str(scene.timeout_duration_secs())
								+ " seconds "
								+ str(scene.parcel_position)
							)
						)
						scene.test_finished = true

				elif Global.scene_runner.is_scene_tests_finished(scene_id):
					scene.test_finished = true
					scene.test_result = Global.scene_runner.get_scene_tests_result(scene_id)
			return

	if dump_test_result_and_get_ok():
		self.exit_gracefully(0)
	else:
		self.exit_gracefully(1)
