class_name ProfilePicture

extends Control

@onready var texture_rect_profile: TextureRect = %TextureRect_Profile
@onready var panel: PanelContainer = %Panel


func async_update_profile_picture(avatar: DclAvatar):
	var avatar_name = avatar.get_avatar_name()
	var nickname_color = avatar.get_nickname_color(avatar_name)
	panel.add_theme_color_override("bg_color", nickname_color)

	var stylebox := panel.get_theme_stylebox("panel")
	stylebox = stylebox.duplicate()
	if stylebox is StyleBoxFlat:
		stylebox.bg_color = nickname_color
	panel.add_theme_stylebox_override("panel", stylebox)


	var avatar_data = avatar.get_avatar_data()
	if avatar_data == null:
		print("Profile picture: avatar_data is null")
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
		print("Profile picture: missing face256 data")
		return
	
	var promise = Global.content_provider.fetch_texture_by_url(hash, url)
	var result = await PromiseUtils.async_awaiter(promise)
	if result is PromiseError:
		printerr("profile_picture::_async_download_image promise error: ", result.get_error())
		return
	texture_rect_profile.texture = result.texture
