extends Node2D

@onready var avatar_node: Avatar = $SubViewportContainer/SubViewport/Avatar
@onready var sub_viewport: SubViewport = $SubViewportContainer/SubViewport

var payload_to_process: AvatarRendererHelper.AvatarFile
var current_payload_index: int = 0

var current_avatar = {
	"base_url": "https://peer.decentraland.org/content",
	"name": "",
	"body_shape": "urn:decentraland:off-chain:base-avatars:BaseFemale",
	"wearables":
	[
		"urn:decentraland:off-chain:base-avatars:f_sweater",
		"urn:decentraland:off-chain:base-avatars:f_jeans",
		"urn:decentraland:off-chain:base-avatars:bun_shoes",
		"urn:decentraland:off-chain:base-avatars:standard_hair",
		"urn:decentraland:off-chain:base-avatars:f_eyes_01",
		"urn:decentraland:off-chain:base-avatars:f_eyebrows_00",
		"urn:decentraland:off-chain:base-avatars:f_mouth_00"
	],
	"eyes": Color(0.3, 0.8, 0.5),
	"hair": Color(0.5960784554481506, 0.37254902720451355, 0.21568627655506134),
	"skin": Color(0.4901960790157318, 0.364705890417099, 0.27843138575553894),
	"emotes": []
}


# TODO: this can be a command line parser and get some helpers like get_string("--realm"), etc
func get_params_from_cmd():
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
	Global.realm.set_realm("null")
	Global.scene_runner.set_pause(true)

	self.start.call_deferred()


func start():
	update_avatar(0)


func update_avatar(index: int):
	var avatar_entry: AvatarRendererHelper.AvatarEntry = payload_to_process.payload[index]
	current_avatar.base_url = payload_to_process.base_url
	current_avatar.body_shape = avatar_entry.avatar.body_shape
	current_avatar.wearables = avatar_entry.avatar.wearables
	current_avatar.skin = avatar_entry.avatar.skin
	current_avatar.hair = avatar_entry.avatar.hair
	current_avatar.eyes = avatar_entry.avatar.eyes

	current_payload_index = index
	avatar_node.update_avatar(current_avatar)


func _on_avatar_avatar_loaded():
	RenderingServer.set_default_clear_color(Color(0, 0, 0, 0))
	await get_tree().process_frame
	await get_tree().process_frame
	await get_tree().process_frame

	var img := sub_viewport.get_texture().get_image()
	img.save_png(payload_to_process.payload[current_payload_index].dest_path)

	if current_payload_index >= payload_to_process.payload.size() - 1:
		get_tree().quit(0)
	else:
		update_avatar(current_payload_index + 1)
