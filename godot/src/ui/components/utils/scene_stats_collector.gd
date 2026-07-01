class_name SceneStatsCollector
extends RefCounted

## Preview-only metric collector for a single scene. Walks the scene's Godot
## subtree to count geometry/colliders, reads entity count from the CRDT, and
## sums on-disk content size. Per-mesh triangle counts and per-hash file sizes
## are cached so repeated refresh ticks stay cheap.

var _tri_cache: Dictionary = {}  # mesh instance_id -> triangles
var _file_size_cache: Dictionary = {}  # content filename -> bytes (immutable once written)


## Clear per-scene caches. Call on scene change. File sizes are immutable on
## disk, so that cache is kept (it is a shared disk-fact cache, not per-scene).
func reset() -> void:
	_tri_cache.clear()


## Per-scene counts for the live tree:
## { triangles, bodies, colliders, entities, geometries, materials, textures }.
func collect_scene(scene_id: int) -> Dictionary:
	var acc: Dictionary = {"triangles": 0, "bodies": 0, "colliders": 0}
	var geos: Dictionary = {}
	var mats: Dictionary = {}
	var texs: Dictionary = {}
	var node: Node = _scene_node(scene_id)
	if node != null:
		_walk(node, acc, geos, mats, texs)
	acc["geometries"] = geos.size()
	acc["materials"] = mats.size()
	acc["textures"] = texs.size()
	acc["entities"] = 0
	if is_instance_valid(Global.scene_runner):
		acc["entities"] = Global.scene_runner.debug_list_entities(scene_id).size()
	return acc


func _scene_node(scene_id: int) -> Node:
	if not is_instance_valid(Global.scene_runner):
		return null
	for child in Global.scene_runner.get_children():
		if child is DclSceneNode and child.get_scene_id() == scene_id:
			return child
	return null


func _walk(
	node: Node, acc: Dictionary, geos: Dictionary, mats: Dictionary, texs: Dictionary
) -> void:
	if node is MeshInstance3D:
		var mi: MeshInstance3D = node
		acc["bodies"] += 1
		var mesh: Mesh = mi.mesh
		if mesh != null:
			geos[mesh.get_instance_id()] = true
			acc["triangles"] += _mesh_triangles(mesh)
			for si in range(mesh.get_surface_count()):
				var mat: Material = mi.get_active_material(si)
				if mat != null:
					mats[mat.get_instance_id()] = true
					_collect_textures(mat, texs)
	elif node is CollisionShape3D:
		acc["colliders"] += 1
	for c in node.get_children():
		_walk(c, acc, geos, mats, texs)


func _collect_textures(mat: Material, texs: Dictionary) -> void:
	if not (mat is BaseMaterial3D):
		return
	var bm: BaseMaterial3D = mat
	var candidates: Array = [
		bm.albedo_texture,
		bm.normal_texture,
		bm.orm_texture,
		bm.metallic_texture,
		bm.roughness_texture,
		bm.emission_texture,
		bm.ao_texture,
		bm.heightmap_texture,
	]
	for tex in candidates:
		if tex != null:
			texs[tex.get_instance_id()] = true


func _mesh_triangles(mesh: Mesh) -> int:
	var id: int = mesh.get_instance_id()
	if _tri_cache.has(id):
		return _tri_cache[id]
	var tris: int = 0
	for si in range(mesh.get_surface_count()):
		var arrays: Array = mesh.surface_get_arrays(si)
		if arrays.is_empty():
			continue
		var indices = arrays[Mesh.ARRAY_INDEX]
		if indices is PackedInt32Array and indices.size() > 0:
			tris += int(indices.size() / 3.0)
		else:
			var verts = arrays[Mesh.ARRAY_VERTEX]
			if verts is PackedVector3Array:
				tris += int(verts.size() / 3.0)
	_tri_cache[id] = tris
	return tris


## Real on-disk content size (bytes) for the scene. Measures what actually lives
## in local storage, which is what a creator cares about — not the manifest size.
##
## A scene asset can persist on disk under several names, all keyed by the same
## content hash:
##   - "{hash}"             raw download (textures, audio, crdt, js, ...)
##   - "{hash}.scn"         runtime-processed GLTF (the raw glb is deleted after)
##   - "{hash}-mobile.zip"  optimized asset pack (used only in optimized mode)
## Summing only the raw "{hash}" (as before) missed the processed .scn files —
## the bulk of a scene's footprint. Here we scan user://content once and add every
## file whose hash (the text before the first '.') is in the scene's content
## mapping: this catches "{hash}" and any "{hash}.ext" processed form, while a
## leftover optimized "{hash}-mobile.zip" is skipped (its prefix "{hash}-mobile"
## is not a bare content hash) so it never double-counts the runtime .scn.
## CID hashes contain no '.', so the split is exact. Recomputed each call from the
## live listing, so it converges as assets download.
func content_bytes(scene_id: int) -> int:
	if scene_id == -1 or not is_instance_valid(Global.scene_fetcher):
		return 0
	var scene_data = Global.scene_fetcher.get_scene_data_by_scene_id(scene_id)
	if scene_data == null or scene_data.scene_entity_definition == null:
		return 0
	var mapping = scene_data.scene_entity_definition.get_content_mapping()
	if mapping == null:
		return 0
	var hashes: Dictionary = {}
	for file in mapping.get_files():
		var content_hash: String = str(mapping.get_hash(file))
		if not content_hash.is_empty():
			hashes[content_hash] = true
	if hashes.is_empty():
		return 0
	return _sum_disk_bytes(hashes)


## Sum the size of every file in user://content whose leading hash is in `hashes`.
func _sum_disk_bytes(hashes: Dictionary) -> int:
	var dir: DirAccess = DirAccess.open("user://content")
	if dir == null:
		return 0
	var total: int = 0
	dir.list_dir_begin()
	var fname: String = dir.get_next()
	while fname != "":
		if not dir.current_is_dir() and hashes.has(_hash_prefix(fname)):
			total += _file_size(fname)
		fname = dir.get_next()
	dir.list_dir_end()
	return total


## Leading content hash of a content filename: the text before the first '.'.
## CID hashes ("bafk...", "bafy...", "Qm...") contain no '.', so this isolates
## the hash for "{hash}" and "{hash}.scn". An optimized "{hash}-mobile.zip"
## yields "{hash}-mobile" (never a bare hash), so it is naturally excluded.
func _hash_prefix(fname: String) -> String:
	var dot: int = fname.find(".")
	if dot == -1:
		return fname
	return fname.substr(0, dot)


## Size of a content file in bytes, cached (file contents are immutable on disk).
func _file_size(fname: String) -> int:
	if _file_size_cache.has(fname):
		return _file_size_cache[fname]
	var f: FileAccess = FileAccess.open("user://content/" + fname, FileAccess.READ)
	if f == null:
		return 0  # transiently unreadable (e.g. being written) — retry next tick
	var size: int = f.get_length()
	f.close()
	_file_size_cache[fname] = size
	return size


## Whole-app render/memory stats. These are engine-global (single shared
## viewport) and CANNOT be attributed to one scene.
static func global_stats() -> Dictionary:
	return {
		"fps": int(Performance.get_monitor(Performance.TIME_FPS)),
		"draw_calls": int(Performance.get_monitor(Performance.RENDER_TOTAL_DRAW_CALLS_IN_FRAME)),
		"texture_vram": int(Performance.get_monitor(Performance.RENDER_TEXTURE_MEM_USED)),
		"video_mem": int(Performance.get_monitor(Performance.RENDER_VIDEO_MEM_USED)),
		"static_mem": int(Performance.get_monitor(Performance.MEMORY_STATIC)),
	}
