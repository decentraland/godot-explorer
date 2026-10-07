extends SceneTree

# One-shot converter: Unity .anim (YAML) -> Godot .tres animations +
# AnimationLibrary, for locomotion clips sourced from unity-explorer's
# AvatarShape/Assets/Animations.
#
# Run headless from the worktree root:
#   .bin/godot/Godot.app/Contents/MacOS/Godot --headless --path godot \
#       --script src/tool/anim_converter/convert_unity_locomotion.gd
#
# Reads:  res://assets/no-export/unity_locomotion/*.anim
#         res://assets/no-export/unity_locomotion/unity_rig_rest.json
#           (Unity prefab rest pose, extracted from AvatarBase.prefab)
#         res://src/decentraland_components/avatar/avatar.tscn
#           (runtime skeleton rest pose — the retarget target)
# Writes: res://assets/animations/unity_locomotion/<clip>.tres
#         res://assets/animations/unity_locomotion.tres  (AnimationLibrary)
#
# Conversion = retargeting, NOT component copy: the Unity rig and our rig are
# the same Blender skeleton mirrored on X (verified: rest transforms match
# under x-mirror), but bone track values in Godot are rest-relative POSES.
# Per bone per key:
#   delta_u(b,t) = W_u_anim(b,t) * W_u_rest(b)^-1      (world rotation delta)
#   W_g(b,t)     = mirror(delta_u) * W_g_rest(b)        (apply on our rest)
#   pose         = rest_g(b)^-1 * local_of(W_g)         (rest-relative track)
# Same for positions (vector mirror, no quaternion).
# Worlds are composed in each rig's own armature/skeleton space (cm both).
# Oracle: compare_final.gd (final-space dot vs locomotion.res must be ~1).

const INPUT_DIR := "res://assets/no-export/unity_locomotion/"
const UNITY_REST_JSON := "res://assets/no-export/unity_locomotion/unity_rig_rest.json"
const AVATAR_SCENE := "res://src/decentraland_components/avatar/avatar.tscn"
const OUTPUT_DIR := "res://assets/animations/unity_locomotion/"
const LIBRARY_PATH := "res://assets/animations/unity_locomotion.tres"
const LOOP_MARKERS := ["_Loop", "Walk", "Jog", "Run", "Idle", "Slide", "point_hand_right_idle"]


class ComponentCurve:
	var times := PackedFloat32Array()
	var values := PackedFloat32Array()

	func sample(t: float, fallback := 0.0) -> float:
		var n := times.size()
		if n == 0:
			return fallback
		if t <= times[0]:
			return values[0]
		if t >= times[n - 1]:
			return values[n - 1]
		for i in range(n - 1):
			if t < times[i + 1]:
				var span := times[i + 1] - times[i]
				var alpha := 0.0 if span <= 0.0 else (t - times[i]) / span
				return lerpf(values[i], values[i + 1], alpha)
		return values[n - 1]


class BoneCurves:
	# attr ("m_LocalRotation.x" / "m_LocalPosition.y") -> ComponentCurve
	var components := {}


class Rig:
	# bone -> {rot: Quaternion, pos: Vector3} local rest, and parent map
	var local_rot := {}
	var local_pos := {}
	var parent := {}
	var order: Array = []  # DFS, root first


var rig_u := Rig.new()  # unity prefab rest
var rig_g := Rig.new()  # avatar.tscn skeleton rest


func _initialize() -> void:
	var out_dir := DirAccess.open("res://assets/animations/")
	if out_dir and not out_dir.dir_exists("unity_locomotion"):
		out_dir.make_dir("unity_locomotion")

	if not _load_unity_rest():
		quit(1)
		return
	if not _load_godot_rest():
		quit(1)
		return
	prints("unity bones:", rig_u.order.size(), "godot bones:", rig_g.order.size())

	var dir := DirAccess.open(INPUT_DIR)
	var library := AnimationLibrary.new()
	var converted := 0
	var only: String = OS.get_environment("ANIM_ONLY")
	for file in dir.get_files():
		if not file.ends_with(".anim"):
			continue
		var clip_name := file.get_basename()
		if not only.is_empty() and clip_name != only:
			continue
		var bones := _parse_runtime_curves(_read(INPUT_DIR + file))
		if bones.is_empty():
			printerr("FAILED (no curves): ", file)
			continue
		var anim := _bake_clip(clip_name, bones)
		if anim == null:
			printerr("FAILED (bake): ", file)
			continue
		var err := ResourceSaver.save(anim, OUTPUT_DIR + clip_name + ".tres")
		if err != OK:
			printerr("SAVE FAILED: ", clip_name, " err=", err)
			continue
		library.add_animation(clip_name, anim)
		converted += 1
		prints(
			"converted", clip_name, "len=%.2f" % anim.length, "tracks=%d" % anim.get_track_count()
		)

	var lib_err := ResourceSaver.save(library, LIBRARY_PATH)
	prints("DONE converted=", converted, "library_err=", lib_err)
	quit(0 if lib_err == OK else 1)


func _read(path: String) -> String:
	var f := FileAccess.open(path, FileAccess.READ)
	if f == null:
		return ""
	var text := f.get_as_text()
	f.close()
	return text


# --- Rig loading ---


func _load_unity_rest() -> bool:
	var text := _read(UNITY_REST_JSON)
	if text.is_empty():
		printerr("missing ", UNITY_REST_JSON)
		return false
	var data: Dictionary = JSON.parse_string(text)
	for name in data:
		var e: Dictionary = data[name]
		if name == "Armature":
			continue  # armature space is the world for both rigs
		var r: Array = e["rot"]
		var p: Array = e["pos"]
		rig_u.local_rot[name] = Quaternion(r[0], r[1], r[2], r[3]).normalized()
		rig_u.local_pos[name] = Vector3(p[0], p[1], p[2])
		rig_u.parent[name] = e["parent"] if e["parent"] != "Armature" else ""
	_dfs_order(rig_u)
	return not rig_u.order.is_empty()


func _load_godot_rest() -> bool:
	var scene: PackedScene = load(AVATAR_SCENE)
	if scene == null:
		printerr("missing ", AVATAR_SCENE)
		return false
	var avatar := scene.instantiate()
	var skel: Skeleton3D = avatar.get_node("Armature/Skeleton3D")
	for i in range(skel.get_bone_count()):
		var name := skel.get_bone_name(i)
		var rest := skel.get_bone_rest(i)
		rig_g.local_rot[name] = rest.basis.get_rotation_quaternion()
		rig_g.local_pos[name] = rest.origin
		var p: int = skel.get_bone_parent(i)
		rig_g.parent[name] = skel.get_bone_name(p) if p >= 0 else ""
	avatar.free()
	_dfs_order(rig_g)
	return not rig_g.order.is_empty()


func _dfs_order(rig: Rig) -> void:
	var roots := []
	for b in rig.parent:
		if rig.parent[b] == "":
			roots.append(b)
	var queue := roots
	while not queue.is_empty():
		var b: String = queue.pop_front()
		rig.order.append(b)
		for child in rig.parent:
			if rig.parent[child] == b:
				queue.append(child)


func _rig_world_rest(rig: Rig) -> Dictionary:
	# bone -> [Quaternion world_rot, Vector3 world_pos]
	var world := {}
	for b in rig.order:
		var p: String = rig.parent[b]
		var pr: Quaternion = world[p][0] if p != "" else Quaternion.IDENTITY
		var pp: Vector3 = world[p][1] if p != "" else Vector3.ZERO
		world[b] = [pr * rig.local_rot[b], pp + pr * rig.local_pos[b]]
	return world


# --- .anim runtime curves ---


func _parse_runtime_curves(text: String) -> Dictionary:
	# Entry layout: `- curve: {keys...}` THEN `path: X` — keys belong to the
	# path that FOLLOWS them, so buffer per entry and flush on `path:`.
	var bones: Dictionary = {}
	var attrs: Array = []
	var pending_times := PackedFloat32Array()
	var pending_values: Array = []
	var pending_time := -1.0
	for line in text.split("\n"):
		if line.begins_with("  m_RotationCurves:"):
			attrs = [
				"m_LocalRotation.x", "m_LocalRotation.y", "m_LocalRotation.z", "m_LocalRotation.w"
			]
			continue
		if line.begins_with("  m_PositionCurves:"):
			attrs = ["m_LocalPosition.x", "m_LocalPosition.y", "m_LocalPosition.z"]
			continue
		if line.begins_with("  m_ScaleCurves:"):
			attrs = []  # bone scale pose not needed
			continue
		if line.begins_with("  m_EulerCurves:") or line.begins_with("  m_FloatCurves:"):
			attrs = []
			continue
		if attrs.is_empty():
			continue
		var trimmed := line.strip_edges()
		if trimmed == "- curve:":
			pending_times = PackedFloat32Array()
			pending_values = []
			for i in range(attrs.size()):
				pending_values.append(PackedFloat32Array())
		elif trimmed.begins_with("time: "):
			pending_time = trimmed.substr(6).to_float()
		elif (
			trimmed.begins_with("value: {")
			and pending_time >= 0.0
			and not pending_values.is_empty()
		):
			var nums := _extract_vec(trimmed)
			if nums.size() == attrs.size():
				pending_times.append(pending_time)
				for i in range(attrs.size()):
					pending_values[i].append(nums[i])
			pending_time = -1.0
		elif trimmed.begins_with("path: "):
			var full_path := trimmed.substr(6)
			var bone := full_path.get_slice("/", full_path.get_slice_count("/") - 1)
			if not bone.is_empty() and not pending_times.is_empty():
				if not bones.has(bone):
					bones[bone] = BoneCurves.new()
				var bc: BoneCurves = bones[bone]
				for i in range(attrs.size()):
					var c := ComponentCurve.new()
					c.times = pending_times
					c.values = pending_values[i]
					bc.components[attrs[i]] = c
			pending_times = PackedFloat32Array()
			pending_values = []
	return bones


func _extract_vec(line: String) -> PackedFloat32Array:
	var out := PackedFloat32Array()
	var body := line.get_slice("{", 1).get_slice("}", 0)
	for part in body.split(","):
		out.append(part.get_slice(":", 1).strip_edges().to_float())
	return out


func _sample_local(bones: Dictionary, b: String, t: float) -> Transform3D:
	if not bones.has(b):
		# No curve: prefab rest local (both maps are cm, same frame modulo mirror).
		if rig_u.local_rot.has(b):
			return Transform3D(Basis(rig_u.local_rot[b]), rig_u.local_pos[b])
		return Transform3D.IDENTITY
	var c: Dictionary = bones[b].components
	var q := (
		Quaternion(
			_sample_attr(c, "m_LocalRotation.x", t),
			_sample_attr(c, "m_LocalRotation.y", t),
			_sample_attr(c, "m_LocalRotation.z", t),
			_sample_attr(c, "m_LocalRotation.w", t, 1.0)
		)
		. normalized()
	)
	var p := Vector3(
		_sample_attr(c, "m_LocalPosition.x", t),
		_sample_attr(c, "m_LocalPosition.y", t),
		_sample_attr(c, "m_LocalPosition.z", t)
	)
	return Transform3D(Basis(q), p)


func _sample_attr(c: Dictionary, attr: String, t: float, fallback := 0.0) -> float:
	var curve: ComponentCurve = c.get(attr)
	return curve.sample(t, fallback) if curve else fallback


# --- Retargeting bake ---


func _mirror_rot(q: Quaternion) -> Quaternion:
	return Quaternion(q.x, -q.y, -q.z, q.w)


func _mirror_pos(p: Vector3) -> Vector3:
	return Vector3(-p.x, p.y, p.z)


func _bake_clip(clip_name: String, bones: Dictionary) -> Animation:
	# Clip key grid = union of all curve times.
	var seen := {}
	for b in bones:
		var bc: BoneCurves = bones[b]
		for attr in bc.components:
			for t in bc.components[attr].times:
				seen[t] = true
	if seen.is_empty():
		return null
	var times := PackedFloat32Array(seen.keys())
	times.sort()

	var wu_rest := _rig_world_rest(rig_u)
	var wg_rest := _rig_world_rest(rig_g)

	# Per time: unity anim world -> mirrored delta -> applied on our rest world.
	# world[b] = [rot, pos] in our skeleton space.
	var anim := Animation.new()
	anim.resource_name = clip_name
	anim.length = times[times.size() - 1]
	anim.step = 1.0 / 30.0
	for marker in LOOP_MARKERS:
		if clip_name == marker or clip_name.ends_with(marker):
			anim.loop_mode = Animation.LOOP_LINEAR
			break

	# Bake per bone (independent tracks, world composed on the fly).
	for b in rig_g.order:
		if not rig_u.local_rot.has(b):
			continue  # bone only in our rig: leave at rest
		var parent: String = rig_g.parent[b]
		var rot_idx := anim.add_track(Animation.TYPE_ROTATION_3D)
		anim.track_set_path(rot_idx, NodePath("Armature/Skeleton3D:" + b))
		var pos_idx := anim.add_track(Animation.TYPE_POSITION_3D)
		anim.track_set_path(pos_idx, NodePath("Armature/Skeleton3D:" + b))
		for t in times:
			# Compose unity anim world for b and its parent chain.
			var wu_b := _unity_world(b, bones, t)
			var delta_rot: Quaternion = _mirror_rot(wu_b[0] * wu_rest[b][0].inverse())
			var wg_rot: Quaternion = delta_rot * wg_rest[b][0]
			var wg_pos: Vector3 = _mirror_pos(wu_b[1] - wu_rest[b][1]) + wg_rest[b][1]
			var local_rot: Quaternion
			var local_pos: Vector3
			if parent == "":
				local_rot = wg_rot
				local_pos = wg_pos
			else:
				var wu_p := _unity_world(parent, bones, t)
				var delta_p: Quaternion = _mirror_rot(wu_p[0] * wu_rest[parent][0].inverse())
				var wg_p_rot: Quaternion = delta_p * wg_rest[parent][0]
				var wg_p_pos: Vector3 = (
					_mirror_pos(wu_p[1] - wu_rest[parent][1]) + wg_rest[parent][1]
				)
				local_rot = wg_p_rot.inverse() * wg_rot
				local_pos = wg_p_rot.inverse() * (wg_pos - wg_p_pos)
			# Bone track values are ABSOLUTE parent-relative transforms (verified
			# against locomotion.res: its tracks store the rest transform itself
			# for unanimated bones), not rest-relative poses.
			anim.rotation_track_insert_key(rot_idx, t, local_rot.normalized())
			anim.position_track_insert_key(pos_idx, t, local_pos)
	return anim


# Unity anim world (armature space) for bone b at time t: compose the chain.
func _unity_world(b: String, bones: Dictionary, t: float) -> Array:
	var chain := []
	var cur := b
	while cur != "":
		chain.push_front(cur)
		cur = rig_u.parent.get(cur, "")
	var rot := Quaternion.IDENTITY
	var pos := Vector3.ZERO
	for node in chain:
		var local := _sample_local(bones, node, t)
		pos = pos + rot * local.origin
		rot = (rot * local.basis.get_rotation_quaternion()).normalized()
	return [rot, pos]
