class_name AvatarMeshAssembler
extends RefCounted

## Assembles wearable meshes onto the avatar's base skeleton: merging extra
## (ADR-316 spring) bones, recycling their slots across outfit changes,
## rebinding mesh skins by name, and converting materials to the shared toon
## shaders. Split out of avatar.gd, which sits against the gdlint
## max-file-lines cap. One instance per Avatar, bound to its base Skeleton3D.

const TOON_SHADER = preload("res://assets/avatar/dcl_toon.gdshader")
const TOON_SHADER_ALPHA_CLIP = preload("res://assets/avatar/dcl_toon_alpha_clip.gdshader")
const TOON_SHADER_ALPHA_BLEND = preload("res://assets/avatar/dcl_toon_alpha_blend.gdshader")
const TOON_SHADER_DOUBLE = preload("res://assets/avatar/dcl_toon_double.gdshader")
const TOON_SHADER_ALPHA_CLIP_DOUBLE = preload(
	"res://assets/avatar/dcl_toon_alpha_clip_double.gdshader"
)
const TOON_SHADER_ALPHA_BLEND_DOUBLE = preload(
	"res://assets/avatar/dcl_toon_alpha_blend_double.gdshader"
)

# The avatar's base skeleton every wearable is merged onto.
var _skeleton: Skeleton3D

# Indices of bones added to the base skeleton by merge_extra_bones and currently
# in use by the active wearables.
var _active_extra_bone_indices: Array[int] = []
# Slots recycled from previous merges (renamed + disabled) waiting to be reused by the next
# merge. Skeleton3D has no remove_bone() in Godot 4.6, so reusing slots is the only way to
# keep bone_count bounded across outfit / body-shape changes.
var _free_bone_pool: Array[int] = []
var _stale_bone_counter: int = 0

# Cache of toon ShaderMaterials keyed by source BaseMaterial3D's instance_id.
# Lets avatars wearing the same wearable share a single ShaderMaterial across
# the whole scene. Skin/hair surfaces clone-on-write in apply_color_and_facial
# so per-avatar tints don't leak.
static var _toon_material_cache: Dictionary = {}

# Issue #1945: matches a Blender-style `_<digits>$` duplicate-import suffix on a
# bone name (e.g. `Avatar_Hips_2`). Compiled once and shared across instances.
static var _bone_suffix_regex: RegEx = RegEx.create_from_string("^(.*)_\\d+$")


func _init(skeleton: Skeleton3D) -> void:
	_skeleton = skeleton


# Renames bones previously merged via merge_extra_bones to a stale placeholder,
# disables them and returns their slots to the pool, so the next merge pass
# starts from a clean slate.
func recycle_extra_bones() -> void:
	if _active_extra_bone_indices.is_empty():
		return
	for bone_idx in _active_extra_bone_indices:
		if bone_idx < 0 or bone_idx >= _skeleton.get_bone_count():
			continue
		var stale_name = "__stale_bone_%d" % _stale_bone_counter
		_stale_bone_counter += 1
		_skeleton.set_bone_name(bone_idx, stale_name)
		_skeleton.set_bone_enabled(bone_idx, false)
		_skeleton.set_bone_parent(bone_idx, -1)
		_skeleton.set_bone_rest(bone_idx, Transform3D.IDENTITY)
		_skeleton.reset_bone_pose(bone_idx)
		_free_bone_pool.push_back(bone_idx)
	_active_extra_bone_indices.clear()


# Resolves a wearable bone name to its counterpart in the base skeleton,
# stripping a Blender-style duplicate-import suffix (`_2`, `_001`, ...) only
# when the un-suffixed name already exists. Returns the original name otherwise
# so genuine extra bones (ADR-316 spring bones) still get merged as new bones.
# Fixes #1945: wearables exported from Blender after re-importing the DCL armature
# carry `Avatar_Hips_2` etc. — without this collapse they merge as a parallel,
# un-animated leg/spine chain that stays in rest pose during emotes/jump/glide.
func resolve_to_base_bone_name(bone_name: String) -> String:
	if _skeleton.find_bone(bone_name) != -1:
		return bone_name
	var m := _bone_suffix_regex.search(bone_name)
	if m == null:
		return bone_name
	var stripped := m.get_string(1)
	if _skeleton.find_bone(stripped) != -1:
		return stripped
	return bone_name


# Copies bones that exist in the wearable's Skeleton3D but not in the base
# (typically ADR-316 spring bones for hair, earrings, capes, etc.). Parents are added
# before children so parent-by-name resolution always succeeds. Without this, mesh
# skins referencing indices beyond the base skeleton's get_bone_count() log
# `Skin bind #N contains bone index bind: N, which is greater than the skeleton bone count`.
func merge_extra_bones(wearable_skel: Skeleton3D) -> void:
	var wearable_bone_count = wearable_skel.get_bone_count()
	if wearable_bone_count == 0:
		return

	# Collect missing bones along with their depth in the wearable hierarchy so we
	# can add parents before children.
	var missing: Array = []  # Array of [depth, wearable_idx, name]
	for i in wearable_bone_count:
		var bone_name = wearable_skel.get_bone_name(i)
		# Skip if the bone already exists in the base, including under its
		# de-suffixed name. The wearable's `Avatar_Hips_2` collapses onto the
		# animated `Avatar_Hips` instead of being merged as a parallel root.
		if resolve_to_base_bone_name(bone_name) != bone_name:
			continue
		if _skeleton.find_bone(bone_name) != -1:
			continue
		var depth = 0
		var cursor = wearable_skel.get_bone_parent(i)
		while cursor != -1:
			depth += 1
			cursor = wearable_skel.get_bone_parent(cursor)
		missing.push_back([depth, i, bone_name])

	if missing.is_empty():
		return

	missing.sort_custom(func(a, b): return a[0] < b[0])

	for entry in missing:
		var wearable_idx: int = entry[1]
		var bone_name: String = entry[2]
		var new_idx: int
		if not _free_bone_pool.is_empty():
			new_idx = _free_bone_pool.pop_back()
			_skeleton.set_bone_name(new_idx, bone_name)
			_skeleton.set_bone_enabled(new_idx, true)
		else:
			new_idx = _skeleton.add_bone(bone_name)
		_skeleton.set_bone_rest(new_idx, wearable_skel.get_bone_rest(wearable_idx))
		_skeleton.reset_bone_pose(new_idx)
		_active_extra_bone_indices.push_back(new_idx)
		# Always reset parent: a recycled slot may have been linked to a stale
		# parent from its previous use. Resolve through the same de-suffix path
		# so a spring bone whose parent is `Avatar_Spine_2` reparents onto the
		# base `Avatar_Spine` instead of leaving as root.
		var parent_wearable_idx = wearable_skel.get_bone_parent(wearable_idx)
		var parent_base_idx = -1
		if parent_wearable_idx != -1:
			var parent_name = wearable_skel.get_bone_name(parent_wearable_idx)
			parent_base_idx = _skeleton.find_bone(resolve_to_base_bone_name(parent_name))
			if parent_base_idx == -1:
				push_warning(
					(
						"[AVATAR] Extra bone '%s' parent '%s' not found in base skeleton; leaving as root"
						% [bone_name, parent_name]
					)
				)
		_skeleton.set_bone_parent(new_idx, parent_base_idx)


# Rewrites a MeshInstance3D's Skin so every bind references its target bone by name.
# Godot resolves named binds against the attached skeleton at runtime, so once the
# mesh is reparented to the base skeleton (which may have been extended with
# extra wearable bones) every joint resolves correctly, including ADR-316 spring bones.
# Issue #1945: when the wearable was exported with duplicate-suffixed bones
# (`Avatar_Hips_2`, `Avatar_LeftLeg_2`, ...), the de-suffix lookup retargets the
# binds onto the animated base bones — without it the mesh tracks merged but
# inert `_2` clones and stays in rest pose during emotes/jump/glide.
func rebind_skin_by_name(mesh: MeshInstance3D, wearable_skel: Skeleton3D) -> void:
	if mesh.skin == null:
		return
	var skin: Skin = mesh.skin.duplicate()
	var wearable_bone_count = wearable_skel.get_bone_count()
	for i in skin.get_bind_count():
		var bone_idx = skin.get_bind_bone(i)
		if bone_idx >= 0 and bone_idx < wearable_bone_count:
			var bone_name = wearable_skel.get_bone_name(bone_idx)
			skin.set_bind_name(i, resolve_to_base_bone_name(bone_name))
	mesh.skin = skin


func apply_toon_material(node_to_apply: Node) -> void:
	if not (node_to_apply is MeshInstance3D) or node_to_apply.mesh == null:
		return
	for surface_idx in range(node_to_apply.mesh.get_surface_count()):
		var mat = node_to_apply.mesh.surface_get_material(surface_idx)
		if mat == null or not (mat is BaseMaterial3D):
			continue
		var key: int = mat.get_instance_id()
		var cached = _toon_material_cache.get(key)
		if cached == null or not is_instance_valid(cached):
			cached = _convert_to_toon(mat)
			_toon_material_cache[key] = cached
		node_to_apply.set_surface_override_material(surface_idx, cached)


func apply_toon_material_recursive(node: Node) -> void:
	apply_toon_material(node)
	for child in node.get_children():
		apply_toon_material_recursive(child)


func _convert_to_toon(base_mat: BaseMaterial3D) -> ShaderMaterial:
	var is_alpha_scissor = base_mat.transparency == BaseMaterial3D.TRANSPARENCY_ALPHA_SCISSOR
	var is_alpha_blend = (
		base_mat.transparency == BaseMaterial3D.TRANSPARENCY_ALPHA
		or base_mat.transparency == BaseMaterial3D.TRANSPARENCY_ALPHA_HASH
		or base_mat.transparency == BaseMaterial3D.TRANSPARENCY_ALPHA_DEPTH_PRE_PASS
	)
	var double_sided = base_mat.cull_mode == BaseMaterial3D.CULL_DISABLED
	var toon_mat = ShaderMaterial.new()
	if is_alpha_scissor and double_sided:
		toon_mat.shader = TOON_SHADER_ALPHA_CLIP_DOUBLE
	elif is_alpha_scissor:
		toon_mat.shader = TOON_SHADER_ALPHA_CLIP
	elif is_alpha_blend and double_sided:
		toon_mat.shader = TOON_SHADER_ALPHA_BLEND_DOUBLE
	elif is_alpha_blend:
		toon_mat.shader = TOON_SHADER_ALPHA_BLEND
	elif double_sided:
		toon_mat.shader = TOON_SHADER_DOUBLE
	else:
		toon_mat.shader = TOON_SHADER
	toon_mat.set_shader_parameter("albedo_color", base_mat.albedo_color)
	if base_mat.albedo_texture:
		toon_mat.set_shader_parameter("albedo_texture", base_mat.albedo_texture)
	if base_mat.emission_enabled:
		toon_mat.set_shader_parameter("emission_color", base_mat.emission)
		if base_mat.emission_texture:
			toon_mat.set_shader_parameter("emission_texture", base_mat.emission_texture)
	if is_alpha_scissor:
		toon_mat.set_shader_parameter("alpha_scissor_threshold", base_mat.alpha_scissor_threshold)
	return toon_mat
