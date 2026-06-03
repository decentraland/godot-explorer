extends Node

# Two-stage, distance-prioritized, budgeted impostor capture queue.
#
# Stage 1 (fetch): cheap, user-visible. Per avatar, closest-in-frustum first:
#   * 0x id -> catalyst body snapshot, uploaded as the impostor.
#   * otherwise -> a stock default body snapshot uploaded immediately as a
#     PLACEHOLDER, and (only when the avatar has custom content worth rendering)
#     the avatar is handed to stage 2 to swap in a real snapshot later.
# Stage 2 (gen): expensive local render (re-assembles the avatar off-screen).
#   Runs only when stage 1 is idle, one at a time, with a frame cooldown, at a
#   reduced resolution, and never for off-frustum avatars. The result swaps the
#   placeholder. See #2206.

const AVATAR_PREVIEW_SCENE = preload("res://src/ui/pages/backpack/avatar_preview.tscn")
const ImpostorCapturePriority = preload(
	"res://src/decentraland_components/avatar/impostor/impostor_capture_priority.gd"
)

# Stock default-avatar profiles the catalyst exposes (profiles/default1..160),
# used as the immediate impostor placeholder.
const DEFAULT_BODY_COUNT := 160

var _fetch_pending: Array = []
var _gen_pending: Array = []
var _enqueued: Dictionary = {}
var _in_progress: bool = false
var _last_gen_frame: int = -100000


func request_capture(avatar) -> void:
	if avatar == null or not is_instance_valid(avatar):
		return
	var iid: int = avatar.get_instance_id()
	if _enqueued.get(iid, false):
		return
	_enqueued[iid] = true
	_fetch_pending.append(avatar)


func _process(_delta: float) -> void:
	if _in_progress:
		return
	# Fetch stage has priority — it's cheap and what the user sees first.
	if not _fetch_pending.is_empty():
		_async_run_fetch()
		return
	# Gen stage: budgeted, with a cooldown so generations never run back-to-back.
	if not _gen_pending.is_empty():
		var since: int = Engine.get_frames_drawn() - _last_gen_frame
		if since >= AvatarImpostorConfig.GEN_MIN_FRAMES_BETWEEN:
			_async_run_gen()


# gdlint:ignore = async-function-name
func _async_run_fetch() -> void:
	_in_progress = true

	var avatar = _take_next(_fetch_pending, false)
	if avatar == null:
		_in_progress = false
		return
	var iid: int = avatar.get_instance_id()

	var got_catalyst := false
	var image: Image = null
	if avatar.avatar_id != "" and avatar.avatar_id.begins_with("0x"):
		image = await _async_fetch_catalyst_body(avatar.avatar_id)
		got_catalyst = image != null
	if image == null:
		# Placeholder for AvatarShapes / unfetchable profiles — never the bake.
		image = await _async_fetch_default_body(avatar)

	if image != null and is_instance_valid(avatar) and Global.avatars != null:
		# The default placeholder must NOT be disk-cached under the avatar's
		# identity key: a persisted placeholder makes request_impostor_layer
		# treat the avatar as already captured (impostor_needs_capture stays
		# false), so the real snapshot would never be generated. Empty key =
		# upload without persisting; only real looks (catalyst / generated)
		# are cached.
		var cache_key: String = _key_for(avatar) if got_catalyst else ""
		Global.avatars.set_impostor_texture(iid, image, cache_key)

	# When the placeholder default stood in for a custom avatar, queue a real
	# snapshot to swap in; otherwise this avatar is done.
	if not got_catalyst and is_instance_valid(avatar) and _should_generate(avatar):
		_gen_pending.append(avatar)
	else:
		_enqueued.erase(iid)

	_in_progress = false


# gdlint:ignore = async-function-name
func _async_run_gen() -> void:
	_in_progress = true

	# Only generate for in-frustum avatars; off-frustum ones stay queued until
	# they come back on screen (Godot already culls their meshes anyway).
	var avatar = _take_next(_gen_pending, true)
	if avatar == null:
		_in_progress = false
		return
	_last_gen_frame = Engine.get_frames_drawn()
	var iid: int = avatar.get_instance_id()

	var image: Image = await _async_capture_local(avatar)
	if image != null and is_instance_valid(avatar) and Global.avatars != null:
		Global.avatars.set_impostor_texture(iid, image, _key_for(avatar))

	_enqueued.erase(iid)
	_in_progress = false


# Pop the highest-priority avatar from `queue`, dropping freed entries. When
# `in_frustum_only` is true, returns null if nothing in-frustum is pending
# (leaving off-frustum entries queued for later).
func _take_next(queue: Array, in_frustum_only: bool):
	var cam: Vector3 = _camera_position()
	var entries: Array = []
	var i := queue.size() - 1
	while i >= 0:
		var a = queue[i]
		if a == null or not is_instance_valid(a):
			queue.remove_at(i)
		i -= 1
	for a in queue:
		entries.append(
			{"distance": cam.distance_to(a.global_position), "off_frustum": a._off_frustum}
		)
	var idx := ImpostorCapturePriority.best_index(entries)
	if idx < 0:
		return null
	if in_frustum_only and entries[idx].off_frustum:
		return null
	var avatar = queue[idx]
	queue.remove_at(idx)
	return avatar


func _camera_position() -> Vector3:
	var viewport := get_viewport()
	var camera := viewport.get_camera_3d() if viewport != null else null
	return camera.global_position if camera != null else Vector3.ZERO


func _key_for(avatar) -> String:
	return avatar._get_impostor_cache_key() if avatar.has_method("_get_impostor_cache_key") else ""


# Real generation only pays off when the avatar has custom content; a plain
# default body already looks identical to the placeholder we uploaded.
# get_wearables() returns a PackedStringArray (not Array), so check it directly.
func _should_generate(avatar) -> bool:
	if avatar.avatar_data == null:
		return false
	return not avatar.avatar_data.get_wearables().is_empty()


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
	# not a bare Texture2D — read the image and skip the placeholder failed marks.
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


# Stable stock-default slot (1..DEFAULT_BODY_COUNT) per visual identity so the
# placeholder doesn't flicker between bodies across recaptures.
func _default_body_slot_for(avatar) -> int:
	var key: String = _key_for(avatar)
	var slot_seed: int = hash(key) if key != "" else avatar.get_instance_id()
	return (abs(slot_seed) % DEFAULT_BODY_COUNT) + 1


# gdlint:ignore = async-function-name
func _async_capture_local(avatar) -> Image:
	if avatar == null or not is_instance_valid(avatar) or avatar.avatar_data == null:
		return null

	var preview: AvatarPreview = AVATAR_PREVIEW_SCENE.instantiate()
	preview.show_platform = false
	preview.hide_name = true
	preview.can_move = false
	# No directional light so the snapshot matches the scene baseline once we
	# render unshaded; the toon shader still emits its baked floor color.
	preview.with_light = false

	var root = get_tree().root
	root.add_child(preview)
	preview.set_position(root.get_visible_rect().size)

	var avatar_name: String = ""
	if avatar.has_method("get_avatar_name"):
		avatar_name = avatar.get_avatar_name()

	await preview.avatar.async_update_avatar(avatar.avatar_data, avatar_name)

	# Reduced resolution; set_impostor_texture upscales to the 256x512 layer.
	var image: Image = await preview.async_get_viewport_image(
		false, AvatarImpostorConfig.GEN_SNAPSHOT_SIZE, 2.5
	)

	preview.queue_free()
	return image
