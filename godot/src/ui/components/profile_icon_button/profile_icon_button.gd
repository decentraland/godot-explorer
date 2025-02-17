extends Control

signal open_profile

@onready var texture_rect_profile = %TextureRect_Profile


func _ready():
	gui_input.connect(self._on_gui_input)
	Global.player_identity.profile_changed.connect(self._async_on_profile_changed)


func _on_gui_input(event: InputEvent):
	if event is InputEventScreenTouch:
		if event.pressed == false:
			open_profile.emit()


func _async_on_profile_changed(new_profile: DclUserProfile):
	var face256_hash = new_profile.get_avatar().get_snapshots_face_hash()
	var face256_url = new_profile.get_avatar().get_snapshots_face_url()
	var promise = Global.content_provider.fetch_texture_by_url(face256_hash, face256_url)
	var result = await PromiseUtils.async_awaiter(promise)
	if result is PromiseError:
		printerr("profile_icon_button::_async_download_image promise error: ", result.get_error())
		return
	texture_rect_profile.texture = result.texture
