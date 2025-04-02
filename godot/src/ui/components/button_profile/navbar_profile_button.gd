extends Button


# gdlint:ignore = async-function-name
func _ready() -> void:
	var profile := Global.player_identity.get_profile_or_null()
	if profile != null:
		await _async_on_profile_changed(profile)
	Global.player_identity.profile_changed.connect(self._async_on_profile_changed)


func _async_on_profile_changed(new_profile: DclUserProfile):
	var face256_hash = new_profile.get_avatar().get_snapshots_face_hash()
	var face256_url = new_profile.get_avatar().get_snapshots_face_url()
	var promise = Global.content_provider.fetch_texture_by_url(face256_hash, face256_url)
	var result = await PromiseUtils.async_awaiter(promise)
	if result is PromiseError:
		printerr("menu_profile_button::_async_download_image promise error: ", result.get_error())
		return
	icon = result.texture


func _on_toggled(toggled_on: bool) -> void:
	$Highlight.visible = toggled_on
