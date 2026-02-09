extends TextureButton

var stylebox: StyleBoxFlat
@onready var texture_rect: TextureRect = %TextureRect
@onready var panel: Panel = %Panel


# gdlint:ignore = async-function-name
func _ready() -> void:
	var profile := Global.player_identity.get_profile_or_null()
	if profile != null:
		await _async_on_profile_changed(profile)
	Global.player_identity.profile_changed.connect(self._async_on_profile_changed)
	Global.snapshot.snapshot_generated.connect(self._on_snapshot_generated)
	stylebox = panel.get_theme_stylebox("panel").duplicate()
	panel.add_theme_stylebox_override("panel", stylebox)


func _on_snapshot_generated(face_image: Image) -> void:
	texture_rect.texture = ImageTexture.create_from_image(face_image)


func _async_on_profile_changed(new_profile: DclUserProfile):
	var face256_hash = new_profile.get_avatar().get_snapshots_face_hash()
	var face256_url = new_profile.get_avatar().get_snapshots_face_url()

	# ADR-290: Snapshots may be empty if profile-images service hasn't generated them yet
	if face256_url.is_empty():
		return

	var promise = Global.content_provider.fetch_texture_by_url(face256_hash, face256_url)
	var result = await PromiseUtils.async_awaiter(promise)
	if result is PromiseError:
		printerr("navbar_profile_button::_async_download_image promise error: ", result.get_error())
		return
	texture_rect.texture = result.texture


func _on_toggled(toggled_on: bool) -> void:
	if toggled_on:
		stylebox.border_width_bottom = 4
		stylebox.border_width_left = 4
		stylebox.border_width_top = 4
		stylebox.border_width_right = 4
		stylebox.border_color = Color("#691FA9")
		Global.open_own_profile.emit()
		Global.send_haptic_feedback()
	else:
		stylebox.set_border_width_all(1)
		stylebox.border_color = Color("#FCFCFC")
		stylebox.border_width_bottom = 1
		stylebox.border_width_left = 1
		stylebox.border_width_top = 1
		stylebox.border_width_right = 1
