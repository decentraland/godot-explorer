class_name AvatarGpuReadyGate
extends RefCounted

## Keeps a freshly assembled remote avatar drawn but fully dithered out until the renderer reports
## every mesh drawable, then fades it in as a whole. The dither is the `own_fade` instance uniform:
## same material, pass and pipelines as the final look, so the hold itself warms them.

const TIMEOUT_SEC := 3.0
const FADE_IN_SEC := 0.2
const DRAW_READY_METHOD := &"instance_geometry_is_draw_ready"

var _generation := 0


## Stops a running gate and shows the avatar; called when a new build begins.
func cancel(avatar: Node) -> void:
	_generation += 1
	var fade := _get_fade(avatar)
	if fade != null:
		fade.set_hold(0.0)


func start(avatar: Node3D, skeleton: Skeleton3D) -> void:
	_generation += 1
	if not FrameWorkBudget.is_background_avatar(avatar):
		return
	if not RenderingServer.has_method(DRAW_READY_METHOD):
		return
	var fade := _get_fade(avatar)
	if fade == null:
		return
	fade.set_hold(1.0)
	_async_run(avatar, skeleton, fade, _generation)


func _async_run(
	avatar: Node3D, skeleton: Skeleton3D, fade: AvatarProximityFade, generation: int
) -> void:
	var tree := avatar.get_tree()
	var started_usec := Time.get_ticks_usec()
	var waited := 0.0
	while true:
		await tree.process_frame
		if generation != _generation or not is_instance_valid(avatar):
			return
		var meshes := _drawn_meshes(skeleton)
		if not _expects_draw(avatar, meshes):
			continue
		var pending := _count_not_ready(meshes)
		if pending > 0:
			waited += avatar.get_process_delta_time()
			if waited < TIMEOUT_SEC:
				continue
		var info := (
			"avatar=%s ms=%d meshes=%d pending=%d"
			% [
				avatar.get("avatar_id"),
				(Time.get_ticks_usec() - started_usec) / 1000,
				meshes.size(),
				pending
			]
		)
		DclProfiler.mark("Avatar::gpu_ready" if pending == 0 else "Avatar::ready_timeout", info)
		break

	var elapsed := 0.0
	while elapsed < FADE_IN_SEC:
		fade.set_hold(1.0 - elapsed / FADE_IN_SEC)
		await tree.process_frame
		if generation != _generation or not is_instance_valid(avatar):
			return
		elapsed += avatar.get_process_delta_time()
	fade.set_hold(0.0)


# Timeout only counts while the meshes would be drawn (on screen, not hidden by LOD).
static func _expects_draw(avatar: Node3D, meshes: Array[MeshInstance3D]) -> bool:
	return not meshes.is_empty() and avatar.is_visible_in_tree() and avatar.get("_on_screen")


static func _drawn_meshes(skeleton: Skeleton3D) -> Array[MeshInstance3D]:
	var meshes: Array[MeshInstance3D] = []
	if not is_instance_valid(skeleton):
		return meshes
	for child in skeleton.get_children():
		if child is MeshInstance3D and child.is_visible_in_tree() and child.mesh != null:
			meshes.push_back(child)
	return meshes


static func _count_not_ready(meshes: Array[MeshInstance3D]) -> int:
	var pending := 0
	for mesh in meshes:
		if not RenderingServer.call(DRAW_READY_METHOD, mesh.get_instance()):
			pending += 1
	return pending


static func _get_fade(avatar: Node) -> AvatarProximityFade:
	if not is_instance_valid(avatar):
		return null
	return avatar.get_node_or_null("AvatarProximityFade") as AvatarProximityFade
