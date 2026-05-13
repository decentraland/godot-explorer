# gdlint: disable=class-definitions-order
## Cell-based visibility culling for the GP benchmark.
##
## Built once after loading_complete. Each static MeshInstance3D is bucketed
## into every cell its world AABB overlaps (not just the center cell, so a
## 60m building correctly remains visible as long as ANY of the cells it
## spans is in the PVS-visible set). Per frame the camera's cell + its 3
## nearest neighbors (chosen by sub-cell position) contribute their PVS
## rows via OR — gives hysteresis at cell boundaries so the visible set
## changes gradually as the camera moves, instead of popping at every 16m
## boundary crossing.
class_name DclVisibilityGrid extends Node

const CELL_SIZE_M: float = 16.0
const MAX_DISTANCE_M: float = 150.0
## Only MIs whose AABB diagonal exceeds this get bucketed into every cell
## they overlap (V3 fix for "big building disappears when its center cell
## is hidden"). Smaller props stay single-cell — bucketing them into
## multiple cells inflates cells_with_content and neutralizes culling.
const MULTI_CELL_BUCKET_THRESHOLD_M: float = 5.0
## Number of frames a cell must consistently fail visibility before its MIs
## actually get hidden. Prevents popping at cell boundaries when the camera
## moves: 0 = immediate hide. >0 = temporal hysteresis. New visibility is
## always immediate (no fade-in delay) so freshly-revealed content shows up
## right away. 60 frames ≈ 1s at 60fps / 2s at 30fps.
const HIDE_DELAY_FRAMES: int = 60
## Cells whose AABB diagonal divided by camera distance is smaller than
## this threshold are hidden — they'd render as a sub-pixel blob.
const MIN_SIZE_DISTANCE_RATIO: float = 0.05
## Blocker selection threshold for the PVS bake.
const BLOCKER_MIN_DIAGONAL_M: float = 8.0
## Eye-height the per-cell PVS rays start from.
const PVS_PLAYER_EYE_HEIGHT: float = 1.5

# ---- Grid topology ----
var built: bool = false
var cell_origin_xz: Vector2 = Vector2.ZERO
var cols: int = 0
var rows: int = 0
# Per-cell flat arrays, indexed by cz * cols + cx.
var _cell_aabb: Array[AABB] = []
var _cell_has_content: PackedByteArray = PackedByteArray()
var _cell_last_visible: PackedByteArray = PackedByteArray()
# Frames the cell has been "should-be-hidden" — used by HIDE_DELAY_FRAMES to
# delay the actual MI flip and prevent popping during camera movement.
var _cell_hide_streak: PackedInt32Array = PackedInt32Array()
# Per-cell list of MI INDICES into _all_mis (PackedInt32Array, fast iter).
var _cell_mi_indices: Array[PackedInt32Array] = []

# ---- Flat MI storage ----
# All static, bucketable MeshInstance3Ds. A single MI may appear in multiple
# cells' _cell_mi_indices when its AABB spans cell borders, but it has ONE
# entry in these flat arrays.
var _all_mis: Array[MeshInstance3D] = []
var _mi_diag_sq: PackedFloat32Array = PackedFloat32Array()
var _mi_center: PackedVector3Array = PackedVector3Array()
# Reference count: how many of this MI's cells are currently visible.
# `mi.visible = (_mi_visible_count[mi_idx] > 0)`.
var _mi_visible_count: PackedInt32Array = PackedInt32Array()

# ---- PVS ----
# Bit `cam_cell * (cols*rows) + target_cell` = 1 when cam_cell can see
# target_cell through the blocker set.
var _pvs_bits: PackedByteArray = PackedByteArray()
var _blockers: Array[AABB] = []
var _pvs_build_ms: int = 0

# ---- Stats ----
var total_mis: int = 0
var multi_cell_mis: int = 0  # MIs that span >1 cell
var cells_with_content: int = 0
var skipped_avatar: int = 0
var skipped_hud: int = 0
var skipped_out_of_grid: int = 0
var skipped_animated: int = 0
var skipped_tween: int = 0
var skipped_modifier: int = 0


func build_from_scene_tree(scene_root: Node) -> Dictionary:
	var mis: Array[MeshInstance3D] = []
	_collect_mis(scene_root, mis)

	# First pass: filter + compute world AABB and bounds.
	var kept: Array[MeshInstance3D] = []
	var kept_world_aabb: Array[AABB] = []
	var min_xz := Vector2(INF, INF)
	var max_xz := Vector2(-INF, -INF)
	for mi in mis:
		var skip_reason := _mi_skip_reason(mi)
		match skip_reason:
			"avatar":
				skipped_avatar += 1
				continue
			"hud":
				skipped_hud += 1
				continue
			"animated":
				skipped_animated += 1
				continue
			"tween":
				skipped_tween += 1
				continue
			"modifier":
				skipped_modifier += 1
				continue
		var t := mi.global_transform
		var world_aabb := _transform_aabb(t, mi.get_aabb())
		var center := world_aabb.position + world_aabb.size * 0.5
		min_xz.x = min(min_xz.x, center.x)
		min_xz.y = min(min_xz.y, center.z)
		max_xz.x = max(max_xz.x, center.x)
		max_xz.y = max(max_xz.y, center.z)
		kept.append(mi)
		kept_world_aabb.append(world_aabb)

	if kept.is_empty():
		built = true
		return _stats_dict()

	cell_origin_xz = Vector2(
		floor(min_xz.x / CELL_SIZE_M) * CELL_SIZE_M, floor(min_xz.y / CELL_SIZE_M) * CELL_SIZE_M
	)
	cols = int(ceil((max_xz.x - cell_origin_xz.x) / CELL_SIZE_M)) + 1
	rows = int(ceil((max_xz.y - cell_origin_xz.y) / CELL_SIZE_M)) + 1
	var n_cells := cols * rows
	_cell_aabb.resize(n_cells)
	_cell_has_content.resize(n_cells)
	_cell_last_visible.resize(n_cells)
	_cell_hide_streak.resize(n_cells)
	_cell_mi_indices.resize(n_cells)
	for i in n_cells:
		_cell_aabb[i] = AABB()
		_cell_has_content[i] = 0
		_cell_last_visible[i] = 1
		_cell_hide_streak[i] = 0
		_cell_mi_indices[i] = PackedInt32Array()

	# Second pass: store flat MI data + bucket each MI into every cell its
	# AABB overlaps. This fixes the "big building disappears when its center
	# cell is hidden" artifact — the MI now lives in all cells it touches.
	for mi_idx in kept.size():
		var mi := kept[mi_idx]
		var world_aabb := kept_world_aabb[mi_idx]
		var center := world_aabb.position + world_aabb.size * 0.5
		var diag_sq := (
			world_aabb.size.x * world_aabb.size.x
			+ world_aabb.size.y * world_aabb.size.y
			+ world_aabb.size.z * world_aabb.size.z
		)
		_all_mis.append(mi)
		_mi_diag_sq.append(diag_sq)
		_mi_center.append(center)
		# Initial visible_count gets bumped per cell membership below.
		_mi_visible_count.append(0)
		var flat_idx := _all_mis.size() - 1

		# Bucket into ONE cell (center) for small MIs, every overlapping cell
		# for large MIs. This keeps the cell-with-content count tight while
		# still preventing big-building-disappears artifacts.
		var diag_xz := sqrt(
			world_aabb.size.x * world_aabb.size.x + world_aabb.size.z * world_aabb.size.z
		)
		var cx_min: int
		var cz_min: int
		var cx_max: int
		var cz_max: int
		if diag_xz >= MULTI_CELL_BUCKET_THRESHOLD_M:
			cx_min = int(floor((world_aabb.position.x - cell_origin_xz.x) / CELL_SIZE_M))
			cz_min = int(floor((world_aabb.position.z - cell_origin_xz.y) / CELL_SIZE_M))
			cx_max = int(
				floor((world_aabb.position.x + world_aabb.size.x - cell_origin_xz.x) / CELL_SIZE_M)
			)
			cz_max = int(
				floor((world_aabb.position.z + world_aabb.size.z - cell_origin_xz.y) / CELL_SIZE_M)
			)
		else:
			var center_cx := int(floor((center.x - cell_origin_xz.x) / CELL_SIZE_M))
			var center_cz := int(floor((center.z - cell_origin_xz.y) / CELL_SIZE_M))
			cx_min = center_cx
			cx_max = center_cx
			cz_min = center_cz
			cz_max = center_cz
		cx_min = clampi(cx_min, 0, cols - 1)
		cz_min = clampi(cz_min, 0, rows - 1)
		cx_max = clampi(cx_max, 0, cols - 1)
		cz_max = clampi(cz_max, 0, rows - 1)
		var cell_count := 0
		for cz in range(cz_min, cz_max + 1):
			for cx in range(cx_min, cx_max + 1):
				var ci := cz * cols + cx
				if _cell_has_content[ci] == 0:
					_cell_aabb[ci] = world_aabb
					_cell_has_content[ci] = 1
					cells_with_content += 1
				else:
					_cell_aabb[ci] = _cell_aabb[ci].merge(world_aabb)
				_cell_mi_indices[ci].append(flat_idx)
				_mi_visible_count[flat_idx] += 1
				cell_count += 1
		if cell_count > 1:
			multi_cell_mis += 1
		total_mis += 1

	_collect_blockers(scene_root)
	_build_pvs()

	built = true
	return _stats_dict()


func update_visibility(camera: Camera3D) -> Dictionary:
	if not built or camera == null:
		return {"toggled_on": 0, "toggled_off": 0, "cells_visible": 0}
	var planes := camera.get_frustum()
	var cam_pos := camera.global_position
	var cam_xz := Vector2(cam_pos.x, cam_pos.z)
	var max_dist_sq := MAX_DISTANCE_M * MAX_DISTANCE_M
	var n := cols * rows
	# Camera cell + 3 nearest neighbor cells (hysteresis to prevent popping
	# as camera crosses cell boundaries). Neighbors picked by which side of
	# the cell the camera is closest to.
	var cam_cells := _camera_cell_neighborhood(cam_pos)
	var toggled_on := 0
	var toggled_off := 0
	var cells_visible := 0
	for idx in n:
		if _cell_has_content[idx] == 0:
			continue
		var aabb := _cell_aabb[idx]
		var cell_center := aabb.position + aabb.size * 0.5
		var dx := cell_center.x - cam_xz.x
		var dz := cell_center.z - cam_xz.y
		var dist_sq := dx * dx + dz * dz
		var visible := dist_sq <= max_dist_sq
		if visible and dist_sq > 4.0:
			var diag_sq := aabb.size.x * aabb.size.x + aabb.size.z * aabb.size.z
			if diag_sq < dist_sq * MIN_SIZE_DISTANCE_RATIO * MIN_SIZE_DISTANCE_RATIO:
				visible = false
		if visible:
			visible = _aabb_in_frustum(aabb, planes)
		# PVS: visible from ANY of the camera's neighborhood cells.
		if visible:
			var pvs_ok := false
			for cc in cam_cells:
				if _pvs_get(cc * n + idx):
					pvs_ok = true
					break
			if not pvs_ok:
				visible = false
		if visible:
			cells_visible += 1
		# Temporal hysteresis on the OFF transition: a cell that just stopped
		# being visible this frame doesn't flip its MIs hidden until it's
		# been not-visible HIDE_DELAY_FRAMES in a row. Hide-counter resets to
		# zero whenever the cell is visible, so a single visible frame
		# restarts the streak.
		var prev := _cell_last_visible[idx] != 0
		if visible:
			_cell_hide_streak[idx] = 0
		else:
			_cell_hide_streak[idx] += 1
			if prev and _cell_hide_streak[idx] < HIDE_DELAY_FRAMES:
				# Treat as still visible for now; skip the flip below.
				continue
		if visible != prev:
			_cell_last_visible[idx] = 1 if visible else 0
			# Multi-cell-aware toggle: update each MI's visible_count and
			# only flip the MeshInstance3D when its count crosses 0.
			var mi_ids := _cell_mi_indices[idx]
			if visible:
				for mid in mi_ids:
					var prev_cnt := _mi_visible_count[mid]
					_mi_visible_count[mid] = prev_cnt + 1
					if prev_cnt == 0:
						var mi := _all_mis[mid]
						if is_instance_valid(mi):
							mi.visible = true
				toggled_on += 1
			else:
				for mid in mi_ids:
					var prev_cnt := _mi_visible_count[mid]
					_mi_visible_count[mid] = prev_cnt - 1
					if prev_cnt == 1:
						var mi := _all_mis[mid]
						if is_instance_valid(mi):
							mi.visible = false
				toggled_off += 1
	return {"toggled_on": toggled_on, "toggled_off": toggled_off, "cells_visible": cells_visible}


## Camera cell. For pinned-pose benches we use just the cell the camera
## currently sits in (single PVS row). For player-controlled production we'd
## OR neighbor cells too — see HIDE_DELAY_FRAMES for the alternative
## artifact-prevention strategy (temporal hysteresis instead of spatial OR).
func _camera_cell_neighborhood(cam_pos: Vector3) -> PackedInt32Array:
	var out := PackedInt32Array()
	var fx := (cam_pos.x - cell_origin_xz.x) / CELL_SIZE_M
	var fz := (cam_pos.z - cell_origin_xz.y) / CELL_SIZE_M
	var cx := clampi(int(floor(fx)), 0, cols - 1)
	var cz := clampi(int(floor(fz)), 0, rows - 1)
	out.append(cz * cols + cx)
	return out


func _build_pvs() -> void:
	var t0 := Time.get_ticks_msec()
	var n := cols * rows
	var n_bits := n * n
	_pvs_bits.resize((n_bits + 7) >> 3)
	for i in _pvs_bits.size():
		_pvs_bits[i] = 0
	for ci in n:
		var ci_center := _cell_geometric_center(ci)
		_pvs_set(ci * n + ci)
		for cj in range(ci + 1, n):
			var cj_center := _cell_geometric_center(cj)
			var blocked := _segment_blocked(ci, cj, ci_center, cj_center)
			if not blocked:
				_pvs_set(ci * n + cj)
				_pvs_set(cj * n + ci)
	_pvs_build_ms = Time.get_ticks_msec() - t0


func _cell_geometric_center(cell_idx: int) -> Vector3:
	if _cell_has_content[cell_idx] != 0:
		var ab := _cell_aabb[cell_idx]
		return ab.position + ab.size * 0.5
	var cx := cell_idx % cols
	var cz := cell_idx / cols
	return Vector3(
		cell_origin_xz.x + (cx + 0.5) * CELL_SIZE_M,
		PVS_PLAYER_EYE_HEIGHT,
		cell_origin_xz.y + (cz + 0.5) * CELL_SIZE_M
	)


func _segment_blocked(ci: int, cj: int, a: Vector3, b: Vector3) -> bool:
	var ab_i := _cell_aabb[ci] if _cell_has_content[ci] != 0 else AABB(a, Vector3(0.1, 0.1, 0.1))
	var ab_j := _cell_aabb[cj] if _cell_has_content[cj] != 0 else AABB(b, Vector3(0.1, 0.1, 0.1))
	for blocker in _blockers:
		if blocker.intersects(ab_i) or blocker.intersects(ab_j):
			continue
		if blocker.intersects_segment(a, b) != null:
			return true
	return false


func _pvs_set(bit_idx: int) -> void:
	_pvs_bits[bit_idx >> 3] |= 1 << (bit_idx & 7)


func _pvs_get(bit_idx: int) -> bool:
	return (_pvs_bits[bit_idx >> 3] & (1 << (bit_idx & 7))) != 0


func _collect_blockers(node: Node) -> void:
	if node is CollisionShape3D:
		var cs := node as CollisionShape3D
		var shape := cs.shape
		if shape != null:
			var local_aabb := AABB()
			if shape is BoxShape3D:
				var sz := (shape as BoxShape3D).size
				local_aabb = AABB(-sz * 0.5, sz)
			elif shape.has_method("get_debug_mesh"):
				local_aabb = shape.get_debug_mesh().get_aabb()
			if local_aabb.size != Vector3.ZERO:
				var world_aabb := _transform_aabb(cs.global_transform, local_aabb)
				if world_aabb.size.length() >= BLOCKER_MIN_DIAGONAL_M:
					_blockers.append(world_aabb)
	for child in node.get_children():
		_collect_blockers(child)


func _collect_mis(node: Node, out: Array[MeshInstance3D]) -> void:
	if node is MeshInstance3D:
		out.append(node)
	for child in node.get_children():
		_collect_mis(child, out)


func _mi_skip_reason(mi: MeshInstance3D) -> String:
	if mi.get_layer_mask() != 1:
		return "hud"
	var n: Node = mi
	while n != null:
		if n is DclAvatar:
			return "avatar"
		if n is AnimationPlayer or n is Skeleton3D:
			return "animated"
		if n.has_meta("dcl_has_tween"):
			return "tween"
		if n.has_meta("dcl_has_modifier"):
			return "modifier"
		n = n.get_parent()
	return ""


func _transform_aabb(t: Transform3D, aabb: AABB) -> AABB:
	var corners := [
		aabb.position,
		aabb.position + Vector3(aabb.size.x, 0, 0),
		aabb.position + Vector3(0, aabb.size.y, 0),
		aabb.position + Vector3(0, 0, aabb.size.z),
		aabb.position + Vector3(aabb.size.x, aabb.size.y, 0),
		aabb.position + Vector3(aabb.size.x, 0, aabb.size.z),
		aabb.position + Vector3(0, aabb.size.y, aabb.size.z),
		aabb.position + aabb.size,
	]
	var first := t * (corners[0] as Vector3)
	var out := AABB(first, Vector3.ZERO)
	for i in range(1, 8):
		out = out.expand(t * (corners[i] as Vector3))
	return out


func _aabb_in_frustum(aabb: AABB, planes: Array[Plane]) -> bool:
	# Godot's `Camera3D.get_frustum()` returns planes oriented OUTWARD.
	# n-vertex test: pick the corner farthest INSIDE; if even that vertex
	# is outside the plane, the whole AABB is outside the frustum.
	for plane in planes:
		var nv := Vector3(
			aabb.position.x + (0.0 if plane.normal.x > 0.0 else aabb.size.x),
			aabb.position.y + (0.0 if plane.normal.y > 0.0 else aabb.size.y),
			aabb.position.z + (0.0 if plane.normal.z > 0.0 else aabb.size.z)
		)
		if plane.distance_to(nv) > 0.0:
			return false
	return true


func _stats_dict() -> Dictionary:
	return {
		"built": built,
		"cols": cols,
		"rows": rows,
		"cells_total": cols * rows,
		"cells_with_content": cells_with_content,
		"total_mis": total_mis,
		"multi_cell_mis": multi_cell_mis,
		"skipped_avatar": skipped_avatar,
		"skipped_hud": skipped_hud,
		"skipped_out_of_grid": skipped_out_of_grid,
		"skipped_animated": skipped_animated,
		"skipped_tween": skipped_tween,
		"skipped_modifier": skipped_modifier,
		"blockers": _blockers.size(),
		"pvs_build_ms": _pvs_build_ms,
		"pvs_bytes": _pvs_bits.size(),
		"cell_origin_x": cell_origin_xz.x,
		"cell_origin_z": cell_origin_xz.y,
	}
