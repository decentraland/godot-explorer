extends Node

# Centralized LOD coordinator. Each Avatar registers on _ready and the
# coordinator ranks them by camera distance every N frames, writing back a
# rank-based cap. Avatar._update_lod takes max(natural_state, rank_cap), so
# the closest MAX_FULL_AVATARS keep FULL, the next MAX_THROTTLED_AVATARS keep
# MID/CROSSFADE, and the rest are forced to FAR even if their natural distance
# would still place them in a higher tier.

var _avatars: Array = []


func register(avatar: Node) -> void:
	if not _avatars.has(avatar):
		_avatars.append(avatar)


func unregister(avatar: Node) -> void:
	_avatars.erase(avatar)


func _process(_delta: float) -> void:
	if _avatars.is_empty():
		return
	if Engine.get_frames_drawn() % AvatarImpostorConfig.DISTANCE_CHECK_PERIOD_FRAMES != 0:
		return

	var viewport := get_viewport()
	if viewport == null:
		return
	var camera := viewport.get_camera_3d()
	if camera == null:
		return
	var cam_pos: Vector3 = camera.global_position

	var entries: Array = []
	for i in range(_avatars.size() - 1, -1, -1):
		var avatar = _avatars[i]
		if not is_instance_valid(avatar):
			_avatars.remove_at(i)
			continue
		if avatar.is_local_player or avatar.hidden or not avatar.avatar_ready:
			avatar._lod_rank_cap = Avatar.LODState.FULL
			continue
		var d: float = cam_pos.distance_to(avatar.global_position)
		entries.append([d, avatar])

	entries.sort_custom(_sort_by_distance)

	var max_full: int = AvatarImpostorConfig.MAX_FULL_AVATARS
	var max_throttled: int = AvatarImpostorConfig.MAX_THROTTLED_AVATARS
	for i in range(entries.size()):
		var avatar = entries[i][1]
		if i < max_full:
			avatar._lod_rank_cap = Avatar.LODState.FULL
		elif i < max_full + max_throttled:
			avatar._lod_rank_cap = Avatar.LODState.MID
		else:
			avatar._lod_rank_cap = Avatar.LODState.FAR


func _sort_by_distance(a: Array, b: Array) -> bool:
	return a[0] < b[0]
