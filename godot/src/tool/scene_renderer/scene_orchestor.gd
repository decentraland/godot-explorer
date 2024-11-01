extends Node

enum PayloadState { NONE = 0, LOADING, PROCESSING, DONE }

const USE_TEST_INPUT = false
const DEFAULT_TIMEOUT_REALM_SECONDS = 15.0
const DEFAULT_TIMEOUT_TEST_SECONDS = 15.0
const DEFAULT_TICK_TO_BE_LOADED = 10

var logs: Array[String] = []

var scenes_to_process: SceneRendererInputHelper.SceneInputFile
var current_payload_index: int = 0
var current_payload_state: PayloadState = PayloadState.NONE
var timeout_count: int = 0
var scene_already_telep: bool = false

var realm_change_emited: bool = false

var test_camera_node: DclCamera3D
var test_player_node: Node3D
var test_camera_tune: bool = false
var test_camera_tune_base_position: Vector3

var are_all_scene_loaded: bool = false


# TODO: this can be a command line parser and get some helpers like get_string("--realm"), etc
func get_params_from_cmd():
	var args := OS.get_cmdline_args()
	var scene_data = null
	var camera_tune := args.find("--test-camera-tune") != -1

	# Only use from the editor
	if USE_TEST_INPUT or args.has("--use-test-input"):
		print("scene-renderer: using test input")
		scene_data = SceneRendererInputHelper.SceneInputFile.from_file_path(
			"res://../tests/scene-renderer-test-input.json"
		)
	else:
		var scene_in_place := args.find("--scene-input-file")

		if scene_in_place != -1 and args.size() > scene_in_place + 1 and scene_data == null:
			var file_path: String = args[scene_in_place + 1]
			prints("scene-renderer: using input file from command line", file_path)
			scene_data = SceneRendererInputHelper.SceneInputFile.from_file_path(file_path)

	return [scene_data, camera_tune]


func _ready():
	print("scene-renderer: running")
	var from_params = get_params_from_cmd()
	if from_params[0] == null:
		printerr("param is missing or wrong, try with --scene-input-file [file]")
		get_tree().quit(1)
		return

	scenes_to_process = from_params[0]
	test_camera_tune = from_params[1]
	if scenes_to_process.scenes.is_empty():
		printerr("no scene input to process")
		get_tree().quit(2)
		return

	Global.get_explorer().disable_move_to = true

	Global.realm.realm_changed.connect(self.on_realm_changed)

	Global.realm.async_set_realm(scenes_to_process.realm_url)
	prints(
		"scene-renderer: realm",
		scenes_to_process.realm_url,
		"- scenes number:",
		scenes_to_process.scenes.size()
	)

	get_tree().create_timer(DEFAULT_TIMEOUT_REALM_SECONDS).timeout.connect(
		self.on_realm_change_timeout
	)

	if test_camera_tune:
		var test_camera_tune_scene = (
			load("res://src/tool/scene_renderer/scene_camera_tune.tscn").instantiate()
		)
		add_child(test_camera_tune_scene)
		test_camera_tune_scene.camera_params_updated.connect(self.on_camera_params_updated)


func on_camera_params_updated(
	type: Camera3D.ProjectionType,
	fov: float,
	ortho_size: float,
	param_position: Vector3,
	param_target: Vector3
) -> void:
	var viewport = get_viewport()
	var camera = viewport.get_camera_3d()

	camera_set(
		camera, type, fov, ortho_size, param_position, param_target, test_camera_tune_base_position
	)


func camera_set(
	camera: Camera3D,
	type: Camera3D.ProjectionType,
	fov: float,
	ortho_size: float,
	param_position: Vector3,
	param_target: Vector3,
	base_position: Vector3
):
	var global_position = base_position + param_position
	var look_at_position = base_position + param_target

	var up = Vector3.UP
	if up.cross(look_at_position - global_position).is_zero_approx():
		up = Vector3.FORWARD

	camera.fov = max(min(fov, 179), 1)
	camera.size = max(ortho_size, 0.001)
	camera.projection = type

	camera.global_position = global_position
	camera.look_at(look_at_position, up)


func on_realm_change_timeout():
	if not realm_change_emited:
		printerr(str(DEFAULT_TIMEOUT_REALM_SECONDS) + " seconds realm changed timeout")
		get_tree().quit(1)
		return

	realm_change_emited = true


func on_realm_changed():
	realm_change_emited = true
	self.process_mode = Node.PROCESS_MODE_ALWAYS

	test_camera_node = Global.scene_runner.camera_node
	test_player_node = Global.scene_runner.player_node

	Global.scene_runner.set_camera_and_player_node(
		test_camera_node, test_player_node, self._on_scene_console_message
	)
	Global.scene_fetcher.set_scene_radius(0)
	Global.comms.change_adapter("offline")


func _on_scene_console_message(scene_id: int, level: int, timestamp: float, text: String) -> void:
	prints("SCENE_LOG", scene_id, level, timestamp, text)


func get_scene_child(scene_id: int) -> DclSceneNode:
	var scene_child: DclSceneNode = null
	for child in Global.scene_runner.get_children():
		var this_scene_progress: float = 0.0
		if child is DclSceneNode:
			if child.get_scene_id() == scene_id:
				scene_child = child

	return scene_child


func _process(_delta):
	are_all_scene_loaded = true

	# tick limiter!
	for child: DclSceneNode in Global.scene_runner.get_children():
		if child.get_last_tick_number() > DEFAULT_TICK_TO_BE_LOADED:
			if not Global.scene_runner.get_scene_is_paused(child.get_scene_id()):
				print(
					"Pausing the scene ", Global.scene_runner.get_scene_title(child.get_scene_id())
				)
				Global.scene_runner.set_scene_is_paused(child.get_scene_id(), true)
		else:
			are_all_scene_loaded = false


func _on_timer_timeout():
	# Continue only when every pointer around was fetched
	if Global.scene_fetcher.scene_entity_coordinator.is_busy():
		return

	if current_payload_index >= scenes_to_process.scenes.size():
		Global.testing_tools.exit_gracefully(0)
		return

	var scene := scenes_to_process.scenes[current_payload_index]
	var scene_id: int = Global.scene_fetcher.get_parcel_scene_id(scene.coords.x, scene.coords.y)
	var scene_child: DclSceneNode = get_scene_child(scene_id)

	match current_payload_state:
		PayloadState.NONE:
			test_player_node.global_position = Vector3(
				scene.coords.x * 16.0 + 8.0, 1.0, -scene.coords.y * 16.0 - 8.0
			)

			test_camera_tune_base_position = Vector3(
				scene.coords.x * 16.0, 0.0, -scene.coords.y * 16.0
			)

			current_payload_state = PayloadState.LOADING
			Global.scene_fetcher.set_scene_radius(scene.scene_distance)
			timeout_count = 0

		PayloadState.LOADING:
			if are_all_scene_loaded:
				current_payload_state = PayloadState.PROCESSING
				if not test_camera_tune:
					if scene.dest_path.ends_with(".png"):
						async_take_camera_photo(scene)
					elif scene.dest_path.ends_with(".glb"):
						async_take_scene_file(scene, scene_child)
					else:
						# do nothing?
						current_payload_state = PayloadState.DONE
				else:
					var camera = FreeLookCamera.new()
					add_child(camera)
					camera.make_current()

					print("ready")
			else:
				timeout_count += 1
				if timeout_count > 10:
					if timeout_count % 10 == 0:
						print("Waiting for scenes:")
						for child: DclSceneNode in Global.scene_runner.get_children():
							var t = child.get_last_tick_number()
							if t > DEFAULT_TICK_TO_BE_LOADED:
								pass
							else:
								print(
									"\t-",
									Global.scene_runner.get_scene_title(child.get_scene_id()),
									t
								)

		PayloadState.DONE:
			current_payload_index += 1
			current_payload_state = PayloadState.NONE


func async_take_scene_file(
	input: SceneRendererInputHelper.SceneRendererInputSpecs, child: DclSceneNode
):
	var pending_promises := Global.content_provider.get_pending_promises()
	if not pending_promises.is_empty():
		await PromiseUtils.async_all(Global.content_provider.get_pending_promises())

	var dest_path = input.dest_path.replacen("$index", str(input.index)).replacen(
		"$coords", str(input.coords.x) + "_" + str(input.coords.y)
	)
	var gltf_document_save := GLTFDocument.new()
	var gltf_state_save := GLTFState.new()
	gltf_document_save.append_from_scene(child, gltf_state_save)
	gltf_document_save.write_to_filesystem(gltf_state_save, dest_path)


func async_take_camera_photo(input: SceneRendererInputHelper.SceneRendererInputSpecs):
	prints("async_take_camera_photo", input)

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
	viewport.size = Vector2i(input.width, input.height)

	var base_position := Vector3(input.coords.x * 16.0, 0.0, -input.coords.y * 16.0)
	var camera_type
	if input.camera.projection == "ortho":
		camera_type = Camera3D.PROJECTION_ORTHOGONAL
	else:
		camera_type = Camera3D.PROJECTION_PERSPECTIVE

	camera_set(
		test_camera_3d,
		camera_type,
		input.camera.fov,
		input.camera.ortho_size,
		input.camera.position,
		input.camera.target,
		base_position
	)

	var explorer = Global.get_explorer()
	explorer.set_visible_ui(false)
	Global.scene_runner.base_ui.visible = false

	# Freeze avatars animation and hide them
	for avatar in Global.avatars.get_children():
		if avatar is Avatar:
			avatar.hide()
			avatar.emote_controller.freeze_on_idle()

	Global.scene_runner.player_node.avatar.emote_controller.freeze_on_idle()
	Global.scene_runner.player_node.avatar.hide()

	await get_tree().process_frame
	await get_tree().process_frame
	await get_tree().process_frame

	var viewport_img := viewport.get_texture().get_image()

	# Test: Uncomment this to see how the snapshot would look like
	# await get_tree().create_timer(10.0).timeout

	get_node("/root/explorer").set_visible_ui(true)
	Global.scene_runner.base_ui.visible = true
	# TODO: should unfreeze avatars?

	viewport.size = previous_viewport_size
	previous_camera.make_current()
	remove_child(test_camera_3d)
	test_camera_3d.queue_free()

	var dest_path = input.dest_path.replacen("$index", str(input.index)).replacen(
		"$coords", str(input.coords.x) + "_" + str(input.coords.y)
	)
	viewport_img.save_png(dest_path)

	current_payload_state = PayloadState.DONE
