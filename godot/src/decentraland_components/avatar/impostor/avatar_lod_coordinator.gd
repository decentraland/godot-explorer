extends Node

# Centralized LOD coordinator. Each Avatar registers on _ready. Every N frames
# the coordinator partitions avatars into in-frustum and off-screen, ranks the
# in-frustum ones by camera distance, and writes back caps. Off-screen avatars
# are forced to FAR + overflow so the closest 8/32 FULL/MID slots go to
# avatars actually visible on screen, not the ones nearest in 3D space (which
# could be behind the camera).

# Bounding sphere around the avatar (center at torso, radius covers head and
# feet plus a little margin). Frustum check pulls a representative point on
# the sphere toward the camera's forward axis and runs Camera3D.is_position_in_frustum
# against it — the surface point closest to the optical axis is the most
# likely to fall inside the frustum, so testing it gives the same answer as a
# full sphere-vs-six-planes test for our purposes, with a single API call
# that doesn't depend on plane-normal conventions.
const FRUSTUM_SPHERE_CENTER_OFFSET: Vector3 = Vector3(0.0, 0.9, 0.0)
const FRUSTUM_SPHERE_RADIUS: float = 1.2

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
		avatar._off_frustum = false
		if avatar.is_local_player or avatar.hidden or not avatar.visible or not avatar.avatar_ready:
			avatar._lod_rank_cap = Avatar.LODState.FULL
			avatar._use_overflow_impostor = false
			continue
		var sphere_center: Vector3 = avatar.global_position + FRUSTUM_SPHERE_CENTER_OFFSET
		var test_point: Vector3 = _sphere_axis_test_point(camera, sphere_center)
		var in_frustum: bool = camera.is_position_in_frustum(test_point)
		if not in_frustum:
			# Off-screen: only flag for slot release. Leave the rank cap on
			# whatever it was last time the avatar was in-frustum, so a
			# briefly off-screen FULL avatar doesn't toggle to FAR and pop
			# its mesh out — Godot's GPU frustum cull already skips drawing
			# off-screen meshes, so leaving the LOD state alone has zero
			# render cost and avoids the visible mesh disappear/reappear.
			avatar._off_frustum = true
			continue
		var d: float = cam_pos.distance_to(avatar.global_position)
		entries.append([d, avatar])

	entries.sort_custom(_sort_by_distance)

	var max_full: int = AvatarImpostorConfig.MAX_FULL_AVATARS
	var max_throttled: int = AvatarImpostorConfig.MAX_THROTTLED_AVATARS
	# Real impostor layers are a finite VRAM resource. Beyond max_full+max_layers,
	# avatars borrow another slot's texture and render fully tinted — looks like
	# a distant silhouette, no capture cost, no LRU thrash.
	var max_real_impostors: int = (
		Global.avatars.impostor_max_layers() if Global.avatars != null else 128
	)
	for i in range(entries.size()):
		var avatar = entries[i][1]
		if i < max_full:
			avatar._lod_rank_cap = Avatar.LODState.FULL
			avatar._use_overflow_impostor = false
		elif i < max_full + max_throttled:
			avatar._lod_rank_cap = Avatar.LODState.MID
			avatar._use_overflow_impostor = false
		else:
			avatar._lod_rank_cap = Avatar.LODState.FAR
			avatar._use_overflow_impostor = i >= max_full + max_real_impostors


func _sort_by_distance(a: Array, b: Array) -> bool:
	return a[0] < b[0]


# Returns the point on the sphere (center, FRUSTUM_SPHERE_RADIUS) closest to
# the camera's forward axis. Testing that single point with
# Camera3D.is_position_in_frustum is enough — if even the closest-to-axis
# surface point falls outside the frustum, the rest of the sphere is too.
# Cheaper than sphere-vs-six-planes and doesn't depend on plane-normal
# conventions.
func _sphere_axis_test_point(camera: Camera3D, sphere_center: Vector3) -> Vector3:
	var cam_pos: Vector3 = camera.global_position
	var fwd: Vector3 = -camera.global_basis.z  # camera looks down -Z in Godot
	var to_center: Vector3 = sphere_center - cam_pos
	var depth: float = to_center.dot(fwd)
	if depth <= 0.0:
		# Sphere center is at/behind the camera — testing center directly is
		# fine, is_position_in_frustum will reject it via the near plane.
		return sphere_center
	var axis_pt: Vector3 = cam_pos + fwd * depth
	var perp: Vector3 = sphere_center - axis_pt
	var perp_len_sq: float = perp.length_squared()
	if perp_len_sq <= FRUSTUM_SPHERE_RADIUS * FRUSTUM_SPHERE_RADIUS:
		# Camera axis passes through (or grazes) the sphere — the axis point
		# at this depth is inside the sphere, ideal for the frustum test.
		return axis_pt
	# Axis misses the sphere; pull center toward axis by exactly the radius.
	return sphere_center - perp.normalized() * FRUSTUM_SPHERE_RADIUS
