class_name ProfilePicture

extends Control

@onready var texture_rect_profile: TextureRect = %TextureRect_Profile


func async_update_profile_picture(avatar: DclAvatar):
	var avatar_data = avatar.get_avatar_data()
	if avatar_data == null:
		print("Profile picture: avatar_data is null")
		return
		
	var face256_hash = avatar_data.get_snapshots_face_hash()
	var face256_url = avatar_data.get_snapshots_face_url()
	
	if face256_hash.is_empty() or face256_url.is_empty():
		print("Profile picture: missing face256 data - hash: '", face256_hash, "', url: '", face256_url, "'")
		return
	
	var promise = Global.content_provider.fetch_texture_by_url(face256_hash, face256_url)
	var result = await PromiseUtils.async_awaiter(promise)
	if result is PromiseError:
		printerr("profile_picture::_async_download_image promise error: ", result.get_error())
		return
	texture_rect_profile.texture = result.texture
