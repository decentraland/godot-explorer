extends Control

@onready var texture_rect_profile: TextureRect = %TextureRect_Profile

func _async_update_profile_picture(profile: DclUserProfile):
	var face256_hash = profile.get_avatar().get_snapshots_face_hash()
	var face256_url = profile.get_avatar().get_snapshots_face_url()
	var promise = Global.content_provider.fetch_texture_by_url(face256_hash, face256_url)
	var result = await PromiseUtils.async_awaiter(promise)
	if result is PromiseError:
		printerr("profile_picture::_async_download_image promise error: ", result.get_error())
		return
	texture_rect_profile.texture = result.texture
