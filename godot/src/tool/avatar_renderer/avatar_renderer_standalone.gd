extends Node2D

const USE_TEST_INPUT = false

var logs: Array[String] = []

var payload_to_process: AvatarRendererHelper.AvatarFile
var current_payload_index: int = 0
var current_avatar = {}

@onready var avatar_node: Avatar = $SubViewportContainer/SubViewport/Avatar
@onready var sub_viewport: SubViewport = $SubViewportContainer/SubViewport
@onready var camera_3d_perpective = %Camera3D_Perpective


# TODO: this can be a command line parser and get some helpers like get_string("--realm"), etc
func get_params_from_cmd():
	if USE_TEST_INPUT:
		return [
			AvatarRendererHelper.AvatarFile.from_file_path(
				"res://src/tool/avatar_renderer/test-input.json"
			)
		]

	var args := OS.get_cmdline_args()
	var avatar_data = null
	var avatar_in_place := args.find("--avatars")

	if avatar_in_place != -1 and args.size() > avatar_in_place + 1:
		var file_path: String = args[avatar_in_place + 1]
		avatar_data = AvatarRendererHelper.AvatarFile.from_file_path(file_path)

	return [avatar_data]


func _ready():
	print("spawning avatar renderer scene")
	var from_params = get_params_from_cmd()
	if from_params[0] == null:
		printerr("param is missing or wrong, try with --avatars [file]")
		get_tree().quit(1)
		return

	payload_to_process = from_params[0]
	if payload_to_process.payload.is_empty():
		printerr("no avatars to process")
		get_tree().quit(2)
		return

	# Disable some functions
	Global.realm.async_set_realm("null")
	Global.scene_runner.set_pause(true)

	Global.realm.content_base_url = payload_to_process.base_url

	self.start.call_deferred()


func start():
	async_update_avatar(0)


func flush_logs():
	for log_item in logs:
		print(log_item)
	logs.clear()


func async_update_avatar(index: int):
	var avatar_entry: AvatarRendererHelper.AvatarEntry = payload_to_process.payload[index]
	current_avatar = avatar_entry.avatar
	current_payload_index = index

	flush_logs()

	if not avatar_entry.entity.is_empty():
		prints("processing payload entity", avatar_entry.entity)
	else:
		prints("processing payload", current_payload_index)

	await get_tree().process_frame

	await avatar_node.async_update_avatar(current_avatar)


func ensure_ends_with(path: String, ends: String) -> String:
	if not path.ends_with(ends):
		return path + ends

	return path


func ensure_base_dir_exists(path: String) -> void:
	var dir = path.get_base_dir()
	if not DirAccess.dir_exists_absolute(dir):
		DirAccess.make_dir_recursive_absolute(dir)


func _async_on_avatar_avatar_loaded():
	var payload := payload_to_process.payload[current_payload_index]
	RenderingServer.set_default_clear_color(Color(0, 0, 0, 0))

	# full body fov 90, y=1
	sub_viewport.size = Vector2(payload.width, payload.height)
	camera_3d_perpective.set_fov(90)
	camera_3d_perpective.position.y = 1.0

	await get_tree().process_frame
	await get_tree().process_frame
	await get_tree().process_frame

	var img := sub_viewport.get_texture().get_image()
	var dest_path := ensure_ends_with(payload.dest_path, ".png")
	ensure_base_dir_exists(dest_path)

	img.save_png(dest_path)
	logs.push_back("🟢 " + dest_path)

	if not payload.face_dest_path.is_empty():
		# face = fov 20, y=1.7
		sub_viewport.size = Vector2(payload.face_width, payload.face_height)
		camera_3d_perpective.set_fov(payload.face_zoom)
		camera_3d_perpective.position.y = 1.75
		await get_tree().process_frame

		var face_img := sub_viewport.get_texture().get_image()
		var face_dest_path := ensure_ends_with(payload.face_dest_path, ".png")
		ensure_base_dir_exists(face_dest_path)

		face_img.save_png(face_dest_path)
		logs.push_back("🟢 " + face_dest_path)

	if current_payload_index >= payload_to_process.payload.size() - 1:
		Global.testing_tools.exit_gracefully(0)
	else:
		async_update_avatar.call_deferred(current_payload_index + 1)
