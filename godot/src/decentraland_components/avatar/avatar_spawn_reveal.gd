class_name AvatarSpawnReveal
extends RefCounted

# #1554: the spawn/teleport hologram — a ghost body under the hologram shader
# while wearables load, then one step into the real meshes. The avatar starts it
# and ticks it; re-triggering mid-reveal restarts the sweep instead of queueing.

enum Phase { IDLE, GHOST_IN, HOLD }

const HOLOGRAM_SHADER := preload("res://assets/shaders/ghost_hologram.gdshader")
const GHOST_BODY_FEMALE := preload("res://assets/avatar/ghost_body_female.glb")
const GHOST_BODY_MALE := preload("res://assets/avatar/ghost_body_male.glb")

const GHOST_IN_DURATION := 0.8
const REVEAL_HEIGHT := 2.1
## After the sweep, hold a random beat so a crowd does not swap in lockstep.
const MIN_HOLD_MIN := 0.25
const MIN_HOLD_MAX := 0.5
## Extra grace for the real meshes after the sweep.
const READY_TIMEOUT := 5.0
## Stop trying to start a reveal this long after the avatar was born.
const GIVE_UP_MS := 10000

var _avatar: Node3D
var _phase: Phase = Phase.IDLE
var _elapsed := 0.0
var _min_hold := 0.0
var _material: ShaderMaterial = null
var _ghost: Node3D = null
var _ghost_skeleton: Skeleton3D = null
var _ghost_meshes: Array[MeshInstance3D] = []
# Bone pairing resolved once per reveal: index i is our bone in _src_bones and
# the matching ghost bone in _dst_bones.
var _src_bones := PackedInt32Array()
var _dst_bones := PackedInt32Array()
# The hologram pipeline is compiled once per process, so warming it once per
# avatar would spawn a pair of skinned ghost bodies per avatar in a spawn burst.
static var _pipeline_warmed := false


func _init(avatar: Node3D) -> void:
	_avatar = avatar


func is_idle() -> bool:
	return _phase == Phase.IDLE


func start() -> void:
	if _avatar.body_shape_skeleton_3d == null:
		return
	if _material == null:
		_material = ShaderMaterial.new()
		_material.shader = HOLOGRAM_SHADER
	# Name resolved (trigger gate) — tint by the profile name color, once.
	var nc := DclAvatar.get_nickname_color(_avatar.avatar_name)
	_material.set_shader_parameter("fresnel_color", Vector4(nc.r, nc.g, nc.b, 0.97))
	end()  # clean any in-progress reveal
	_ghost_meshes.clear()
	# The ghost is the NAKED BASE BODY of the avatar's body type: the bundled
	# GLB instanced whole, with its own armature (reparenting a mesh onto our
	# skeleton does not engage skinning). Bone poses copy from our skeleton.
	var body_type: String = (
		_avatar.avatar_data.get_body_shape().to_lower() if _avatar.avatar_data != null else ""
	)
	_ghost = (
		GHOST_BODY_MALE.instantiate()
		if "female" not in body_type
		else GHOST_BODY_FEMALE.instantiate()
	)
	_ghost.name = "__ghost"
	# The ghost GLB faces +Z; our avatar rig faces -Z — flip it.
	_ghost.rotation.y = PI
	_avatar.add_child(_ghost)
	_ghost_skeleton = _find_skeleton_in(_ghost)
	if _ghost_skeleton != null:
		for child in _ghost_skeleton.get_children():
			if child is MeshInstance3D:
				child.material_override = _material
				_ghost_meshes.append(child)
			# Drive off OUR skeleton's rest proportions (same rig).
			_ghost_skeleton.scale = Vector3.ONE
		_resolve_ghost_bones()
	_avatar.body_shape_skeleton_3d.visible = false
	_phase = Phase.GHOST_IN
	_elapsed = 0.0
	_min_hold = randf_range(MIN_HOLD_MIN, MIN_HOLD_MAX)


# One-time warm-up so the first real spawn hologram never renders as Godot's
# solid placeholder while the shader pipeline compiles.
func async_warm_shader() -> void:
	if _pipeline_warmed:
		return
	_pipeline_warmed = true
	var mat := ShaderMaterial.new()
	mat.shader = HOLOGRAM_SHADER
	var warmers: Array[Node3D] = []
	for scene in [GHOST_BODY_FEMALE, GHOST_BODY_MALE]:
		var inst: Node3D = scene.instantiate()
		inst.scale = Vector3(0.001, 0.001, 0.001)
		_avatar.add_child(inst)
		warmers.append(inst)
		var skel := _find_skeleton_in(inst)
		if skel != null:
			for c in skel.get_children():
				if c is MeshInstance3D:
					c.material_override = mat
	await _avatar.get_tree().create_timer(0.5).timeout
	for inst in warmers:
		if is_instance_valid(inst):
			inst.queue_free()


func end() -> void:
	# Clean instant swap: ghost out, real avatar meshes on (one switch).
	if _ghost != null:
		if is_instance_valid(_ghost):
			_ghost.queue_free()
		_ghost = null
		_ghost_skeleton = null
	_ghost_meshes.clear()
	_src_bones = PackedInt32Array()
	_dst_bones = PackedInt32Array()
	_avatar.body_shape_skeleton_3d.visible = true
	_phase = Phase.IDLE
	# A capture asked for while the ghost was up was skipped (see
	# Avatar._update_impostor), so ask again now. Never invalidate here: captures
	# mid-reveal are already blocked, so there is no texture to throw away.
	if Global.avatars != null and Global.avatars.impostor_needs_capture(_avatar.get_instance_id()):
		ImpostorCapturer.request_capture(_avatar)


func tick(delta: float) -> void:
	if _phase == Phase.IDLE:
		return
	_elapsed += delta
	# Per-frame: keep the hologram override (the assembler stomps overrides), and
	# mirror our skeleton's pose onto the ghost so it animates with us.
	for mesh in _ghost_meshes:
		if is_instance_valid(mesh):
			mesh.material_override = _material
	_mirror_pose_to_ghost()
	match _phase:
		Phase.GHOST_IN:
			var progress := clampf(_elapsed / GHOST_IN_DURATION, 0.0, 1.0)
			_material.set_shader_parameter(
				"reveal_y", _avatar.global_position.y - 0.05 + progress * REVEAL_HEIGHT
			)
			if progress >= 1.0:
				_phase = Phase.HOLD
		Phase.HOLD:
			_material.set_shader_parameter("reveal_y", _avatar.global_position.y + REVEAL_HEIGHT)
			var ready: bool = _avatar.avatar_ready or _elapsed >= GHOST_IN_DURATION + READY_TIMEOUT
			var held := _elapsed >= GHOST_IN_DURATION + _min_hold
			if ready and held:
				end()


# Copies our skeleton's pose onto the ghost body's skeleton (same rig, bone names
# match) so the spawn hologram animates with the avatar.
func _mirror_pose_to_ghost() -> void:
	if _ghost_skeleton == null or not is_instance_valid(_ghost_skeleton):
		return
	for i in _src_bones.size():
		var ghost_bone: int = _dst_bones[i]
		var ours: int = _src_bones[i]
		_ghost_skeleton.set_bone_pose_position(
			ghost_bone, _avatar.body_shape_skeleton_3d.get_bone_pose_position(ours)
		)
		_ghost_skeleton.set_bone_pose_rotation(
			ghost_bone, _avatar.body_shape_skeleton_3d.get_bone_pose_rotation(ours)
		)


# Ghost bone -> our bone. Doing it per frame meant a String allocation
# (get_bone_name) plus a name hash (find_bone) for every bone, which cost more
# than the pose copy it was guarding.
func _resolve_ghost_bones() -> void:
	_src_bones = PackedInt32Array()
	_dst_bones = PackedInt32Array()
	for i in _ghost_skeleton.get_bone_count():
		var ours: int = _avatar.body_shape_skeleton_3d.find_bone(_ghost_skeleton.get_bone_name(i))
		if ours < 0:
			continue
		_dst_bones.append(i)
		_src_bones.append(ours)


# #1554: first Skeleton3D in a ghost body GLB instance.
func _find_skeleton_in(node: Node) -> Skeleton3D:
	if node is Skeleton3D:
		return node
	for child in node.get_children():
		var found := _find_skeleton_in(child)
		if found != null:
			return found
	return null
