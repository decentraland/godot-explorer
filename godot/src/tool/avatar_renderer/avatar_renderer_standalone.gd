extends Node

const USE_TEST_INPUT = false

var logs: Array[String] = []

var profiles_to_process: AvatarRendererHelper.AvatarFile
var current_profile_index: int = 0
var current_avatar: DclAvatarWireFormat

@onready var avatar_preview = %AvatarPreview


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

	profiles_to_process = from_params[0]
	if profiles_to_process.profiles.is_empty():
		printerr("no avatars to process")
		get_tree().quit(2)
		return

	# Disable some functions
	#Global.realm.async_set_realm("null")
	Global.scene_runner.set_pause(true)

	Global.realm.content_base_url = profiles_to_process.base_url

	self.start.call_deferred()


func start():
	async_update_avatar(0)


func flush_logs():
	for log_item in logs:
		print(log_item)
	logs.clear()


func async_update_avatar(index: int):
	var profile: AvatarRendererHelper.AvatarRendererSpecs = profiles_to_process.profiles[index]

	current_avatar = profile.avatar
	current_profile_index = index

	flush_logs()

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
			true, Vector2i(profile.face_width, profile.face_height)
		)
		face_image.save_png(face_dest_path)
		logs.push_back("🟢 " + face_dest_path)

	if current_profile_index >= profiles_to_process.profiles.size() - 1:
		Global.testing_tools.exit_gracefully(0)
	else:
		async_update_avatar.call_deferred(current_profile_index + 1)
