extends Button

signal mention_selected(avatar_name: String)

var _avatar_name: String

@onready var profile_picture: ProfilePicture = %ProfilePicture
@onready var label_nickname: Label = %Nickname
@onready var texture_rect_checkmark: TextureRect = %TextureRect_Checkmark


func setup(avatar) -> void:
	_avatar_name = avatar.get_avatar_name()
	if _avatar_name.is_empty():
		return

	var display_name := _avatar_name
	var has_claimed := !_avatar_name.contains("#")
	var tag_pos := display_name.find("#")
	if tag_pos != -1:
		display_name = display_name.left(tag_pos)
	label_nickname.text = display_name

	var color := DclAvatar.get_nickname_color(_avatar_name)
	if label_nickname.label_settings:
		var settings := label_nickname.label_settings.duplicate()
		settings.font_color = color
		label_nickname.label_settings = settings
	else:
		label_nickname.add_theme_color_override("font_color", color)
	texture_rect_checkmark.visible = has_claimed

	var avatar_data = avatar.get_avatar_data()
	if avatar_data:
		var social_data := SocialItemData.new()
		social_data.name = _avatar_name
		social_data.address = avatar.avatar_id
		social_data.profile_picture_url = avatar_data.get_snapshots_face_url()
		social_data.has_claimed_name = has_claimed
		profile_picture.async_update_profile_picture(social_data)


func _on_pressed() -> void:
	mention_selected.emit(_avatar_name)
