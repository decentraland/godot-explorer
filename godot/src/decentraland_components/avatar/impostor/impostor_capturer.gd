extends Node

const AVATAR_PREVIEW_SCENE = preload("res://src/ui/components/backpack/avatar_preview.tscn")

var _queue: Array = []
var _enqueued: Dictionary = {}
var _in_progress: bool = false


func request_capture(avatar) -> void:
	if avatar == null or not is_instance_valid(avatar):
		return
	var iid: int = avatar.get_instance_id()
	if _enqueued.get(iid, false):
		return
	_enqueued[iid] = true
	_queue.append(avatar)


func _process(_delta: float) -> void:
	if _in_progress or _queue.is_empty():
		return
	_async_process_next()


# gdlint:ignore = async-function-name
func _async_process_next() -> void:
	_in_progress = true

	var avatar = _queue.pop_front()
	if avatar == null or not is_instance_valid(avatar):
		_in_progress = false
		return

	var iid: int = avatar.get_instance_id()
	_enqueued.erase(iid)

	var image: Image = await _async_get_image_for(avatar)
	if image != null and is_instance_valid(avatar) and Global.avatars != null:
		Global.avatars.set_impostor_texture(iid, image)

	_in_progress = false


# gdlint:ignore = async-function-name
func _async_get_image_for(avatar) -> Image:
	if avatar.avatar_id != "" and avatar.avatar_id.begins_with("0x"):
		var image := await _async_fetch_catalyst_body(avatar.avatar_id)
		if image != null:
			return image
	return await _async_capture_local(avatar)


# gdlint:ignore = async-function-name
func _async_fetch_catalyst_body(user_id: String) -> Image:
	if Global.content_provider == null:
		return null
	var promise: Promise = Global.content_provider.fetch_avatar_body_texture(user_id)
	if promise == null:
		return null
	var result = await PromiseUtils.async_awaiter(promise)
	if result is PromiseError:
		return null
	if result is Texture2D:
		return result.get_image()
	return null


# gdlint:ignore = async-function-name
func _async_capture_local(avatar) -> Image:
	if avatar == null or not is_instance_valid(avatar) or avatar.avatar_data == null:
		return null

	var preview: AvatarPreview = AVATAR_PREVIEW_SCENE.instantiate()
	preview.show_platform = false
	preview.hide_name = true
	preview.can_move = false

	var root = get_tree().root
	root.add_child(preview)
	preview.set_position(root.get_visible_rect().size)

	var avatar_name: String = ""
	if avatar.has_method("get_avatar_name"):
		avatar_name = avatar.get_avatar_name()

	await preview.avatar.async_update_avatar(avatar.avatar_data, avatar_name)

	var image: Image = await preview.async_get_viewport_image(
		false, AvatarImpostorConfig.TEXTURE_SIZE, 2.5
	)
	preview.queue_free()
	return image
