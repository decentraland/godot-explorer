extends Node

# Number of stock default-avatar profiles the catalyst exposes
# (profiles/default1 .. profiles/default160).
const DEFAULT_BODY_COUNT := 160

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
		var key: String = (
			avatar._get_impostor_cache_key() if avatar.has_method("_get_impostor_cache_key") else ""
		)
		Global.avatars.set_impostor_texture(iid, image, key)

	_in_progress = false


# gdlint:ignore = async-function-name
func _async_get_image_for(avatar) -> Image:
	if avatar.avatar_id != "" and avatar.avatar_id.begins_with("0x"):
		var image := await _async_fetch_catalyst_body(avatar.avatar_id)
		if image != null:
			return image
	# Fallback for AvatarShapes / profiles we can't fetch: a stock default body
	# snapshot. We deliberately never run the old main-thread local bake here —
	# it blocks the frame. Re-enabling real generation behind an async queue is
	# tracked in #2206.
	return await _async_fetch_default_body(avatar)


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
	# fetch_avatar_body_texture resolves a TextureEntry (image + texture + failed),
	# not a bare Texture2D. The old `is Texture2D` check never matched, so every
	# catalyst body — even when the download succeeded — was discarded and fell
	# back to the ~300ms off-screen local bake. Read the image directly and skip
	# the placeholder that TextureEntry.failed marks.
	if result != null and result.get("image") != null and not result.get("failed"):
		return result.image
	return null


# gdlint:ignore = async-function-name
func _async_fetch_default_body(avatar) -> Image:
	if Global.content_provider == null:
		return null
	var promise: Promise = Global.content_provider.fetch_default_avatar_body_texture(
		_default_body_slot_for(avatar)
	)
	if promise == null:
		return null
	var result = await PromiseUtils.async_awaiter(promise)
	if result is PromiseError:
		return null
	if result != null and result.get("image") != null and not result.get("failed"):
		return result.image
	return null


# Pick a stock-default slot (1..DEFAULT_BODY_COUNT) stable per visual identity so
# the same avatar keeps the same default across recaptures instead of flickering
# between bodies. Falls back to the instance id when there's no cache key.
func _default_body_slot_for(avatar) -> int:
	var key: String = (
		avatar._get_impostor_cache_key() if avatar.has_method("_get_impostor_cache_key") else ""
	)
	var slot_seed: int = hash(key) if key != "" else avatar.get_instance_id()
	return (abs(slot_seed) % DEFAULT_BODY_COUNT) + 1
