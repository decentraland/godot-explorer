extends Control

var avatar_list: Array = []

@onready var sub_viewport_container = $SubViewportContainer
@onready var avatar: Avatar = sub_viewport_container.avatar
@onready var emote_wheel = $TabContainer/Emotes/EmoteWheel

@onready var text_edit_expr = $TabContainer/Expression/VBoxContainer/TextEdit_Expr
@onready var text_edit_result = $TabContainer/Expression/VBoxContainer/TextEdit_Result
@onready var line_edit_custom = $TabContainer/Emotes/LineEdit_Custom
@onready var option_button_avatar_list = $TabContainer/Avatars/OptionButton_AvatarList


func _ready():
	load_avatar_list()
	avatar.avatar_loaded.connect(self._on_avatar_loaded)
	option_button_avatar_list.selected = -1
	option_button_avatar_list.text = "Select an avatar"


func load_avatar_list():
	var file = FileAccess.open("res://src/test/avatar/avatar_list.json", FileAccess.READ)
	if file == null:
		printerr("the file does not exist")
		return

	var json_value = JSON.parse_string(file.get_as_text())
	if json_value == null or not json_value is Dictionary:
		printerr("the file has to be a valid json dictionary")
		return

	avatar_list = json_value.get("avatars", [])
	for avatar_i in avatar_list.size():
		option_button_avatar_list.add_item(avatar_list[avatar_i].ref, avatar_i)


func download_wearable(id: String):
	var wearable = Global.content_provider.get_wearable(id)
	var dir_name = "user://downloaded/" + wearable.get_display_name().validate_filename()
	var content_mapping := wearable.get_content_mapping()

	DirAccess.make_dir_recursive_absolute(dir_name)

	for file_name in content_mapping.get_files():
		var file_hash = content_mapping.get_hash(file_name)
		var file_path = dir_name + "/" + file_name.validate_filename()
		if FileAccess.file_exists("user://content/" + file_hash):
			DirAccess.copy_absolute("user://content/" + file_hash, file_path)


func download_wearable_json(id: String):
	var wearable = Global.content_provider.get_wearable(id)
	return JSON.parse_string(wearable.to_json_string())


func download_wearables_avatar_json():
	var items = []
	items.push_back(download_wearable_json(avatar.avatar_data.get_body_shape()))
	for wearable_id in avatar.avatar_data.get_wearables():
		items.push_back(download_wearable_json(wearable_id))

	DisplayServer.clipboard_set(JSON.stringify({"wearables": items}, "\t"))


func download_avatar():
	download_wearable(avatar.avatar_data.get_body_shape())
	for wearable_id in avatar.avatar_data.get_wearables():
		download_wearable(wearable_id)


func _on_avatar_loaded():
	pass


func _on_button_open_wheel_pressed():
	emote_wheel.show()


func _on_text_edit_expr_text_changed():
	var expression = Expression.new()
	var err = expression.parse(text_edit_expr.text, ["Global"])

	if err != OK:
		text_edit_result.text = "Parse failed: " + expression.get_error_text()
		return

	var result = expression.execute([Global], self)
	if expression.has_execute_failed():
		text_edit_result.text = "Execution failed: " + expression.get_error_text()
		return

	text_edit_result.text = "Ok: " + str(result)


func _on_button_play_custom_pressed():
	avatar.emote_controller.async_play_emote(line_edit_custom.text)


func _on_button_clear_pressed():
	avatar.emote_controller.clean_unused_emotes()


# gdlint:ignore = async-function-name
func _on_option_button_avatar_list_item_selected(index):
	var avatar_i = option_button_avatar_list.get_item_id(index)

	var profile: DclUserProfile = DclUserProfile.new()
	var avatar_wf: DclAvatarWireFormat = profile.get_avatar()

	avatar_wf.set_wearables(PackedStringArray(avatar_list[avatar_i].wearables))
	avatar_wf.set_force_render(avatar_list[avatar_i].forceRender)
	avatar_wf.set_body_shape(avatar_list[avatar_i].bodyShape)

	await avatar.async_update_avatar(avatar_wf, "")


func _on_button_download_wearables_pressed():
	download_avatar()


func _on_button_copy_wearable_data_pressed():
	download_wearables_avatar_json()


func _on_button_refresh_pressed():
	_on_option_button_avatar_list_item_selected(option_button_avatar_list.selected)
