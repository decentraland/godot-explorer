class_name ProfilePicture

extends Control

@onready var texture_rect_profile: TextureRect = %TextureRect_Profile
@onready var panel_border: PanelContainer = %Panel_Border
@onready var panel: PanelContainer = %Panel


func async_update_profile_picture(avatar: DclAvatar):
	var avatar_name = avatar.get_avatar_name()
	var nickname_color = avatar.get_nickname_color(avatar_name)

	var background_color = nickname_color

	var stylebox_background := panel.get_theme_stylebox("panel")
	stylebox_background = stylebox_background.duplicate()
	if stylebox_background is StyleBoxFlat:
		stylebox_background.bg_color = background_color
	panel.add_theme_stylebox_override("panel", stylebox_background)

	var white = Color.WHITE
	var factor = 0.3
	var border_color = background_color.lerp(white, factor)

	var stylebox_border := panel_border.get_theme_stylebox("panel")
	stylebox_border = stylebox_border.duplicate()
	if stylebox_border is StyleBoxFlat:
		stylebox_border.border_color = border_color
	panel_border.add_theme_stylebox_override("panel", stylebox_border)

	var avatar_data = avatar.get_avatar_data()
	if avatar_data == null:
		printerr("Profile picture: avatar_data is null")
		return

	var face256_value = avatar_data.to_godot_dictionary()["snapshots"]["face256"]
	var hash = ""
	var url = ""
	if face256_value.begins_with("http"):
		var parts = face256_value.split("/")
		hash = parts[4]
		url = face256_value
	else:
		hash = face256_value
		url = "https://profile-images.decentraland.org/entities/%s/face.png" % hash

	if hash.is_empty() or url.is_empty():
		printerr("Profile picture: missing face256 data")
		return

	var promise = Global.content_provider.fetch_texture_by_url(hash, url)
	var result = await PromiseUtils.async_awaiter(promise)
	if result is PromiseError:
		printerr("profile_picture::_async_download_image promise error: ", result.get_error())
		return
	texture_rect_profile.texture = result.texture
