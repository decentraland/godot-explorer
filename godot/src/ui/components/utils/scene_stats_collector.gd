class_name SceneStatsCollector
extends RefCounted

## Preview-only metric collector for a single scene. Counts scene resources via
## the shared DebugCollector walker, and sums cached content sizes from the
## ResourceProvider's in-memory metadata. Per-mesh triangle counts are cached
## here so repeated refresh ticks stay cheap.

var _tri_cache: Dictionary = {}  # mesh instance_id -> triangles


## Clear per-scene caches. Call on scene change.
func reset() -> void:
	_tri_cache.clear()


## Per-scene counts for the live tree:
## { triangles, bodies, colliders, entities, geometries, materials, textures }.
## Thin wrapper over the shared DebugCollector walker; this instance only owns
## the triangle cache.
func collect_scene(scene_id: int) -> Dictionary:
	return DebugCollector.collect_scene_resources(scene_id, _tri_cache)


## Real local content size (bytes) for the scene — what actually lives in the
## download cache, which is what a creator cares about, not the manifest size.
##
## A scene asset can persist under several names, all keyed by the same content
## hash: "{hash}" (raw download), "{hash}.scn" (runtime-processed GLTF) and
## "{hash}-mobile.zip" (optimized asset pack). The base-name matching that ties
## them back to a mapping hash lives Rust-side (resource_provider.rs
## cache_file_base_name), and sizes come from the ResourceProvider's in-memory
## cache metadata — no disk I/O. Recomputed each call, so it converges as
## assets download.
func content_bytes(scene_id: int) -> int:
	if scene_id == -1 or not is_instance_valid(Global.scene_fetcher):
		return 0
	var scene_data = Global.scene_fetcher.get_scene_data_by_scene_id(scene_id)
	if scene_data == null or scene_data.scene_entity_definition == null:
		return 0
	var mapping = scene_data.scene_entity_definition.get_content_mapping()
	if mapping == null:
		return 0
	var hashes := PackedStringArray()
	for file in mapping.get_files():
		var content_hash: String = str(mapping.get_hash(file))
		if not content_hash.is_empty():
			hashes.append(content_hash)
	if hashes.is_empty():
		return 0
	return Global.content_provider.get_cache_size_for_base_names(hashes)


## External (non-deployed) content the scene pulled at runtime: url-sourced
## textures cached as "hashed_{hex}[_q{N}]" (sizes from the ResourceProvider's
## in-memory cache metadata, so the value converges as downloads land) plus
## bytes the scene's JS consumed via fetch() (never stored on disk). External
## video is streamed and not included. Rust side: content/external_content.rs
## registry, exposed by SceneManager.get_scene_external_content().
func external_bytes(scene_id: int) -> int:
	if scene_id == -1 or not is_instance_valid(Global.scene_runner):
		return 0
	var info: Dictionary = Global.scene_runner.get_scene_external_content(scene_id)
	var total: int = int(info.get("fetch_bytes", 0))
	var files: PackedStringArray = info.get("files", PackedStringArray())
	if files.is_empty():
		return total
	return total + Global.content_provider.get_cache_size_for_base_names(files)


## Whole-app render/memory stats. These are engine-global (single shared
## viewport) and CANNOT be attributed to one scene.
static func global_stats() -> Dictionary:
	return {
		"fps": int(Performance.get_monitor(Performance.TIME_FPS)),
		"draw_calls": int(Performance.get_monitor(Performance.RENDER_TOTAL_DRAW_CALLS_IN_FRAME)),
		"static_mem": _process_memory_bytes(),
	}


## Real process memory (resident set) in bytes, cross-platform. Godot's
## Performance.MEMORY_STATIC only tracks the engine's OWN allocator and is
## compiled out of release / mobile export templates — it returns 0 on mobile,
## which is the "CPU memory shows 0" bug. Prefer the OS-level RSS/footprint the
## Rust side reads (SceneManager.get_process_memory_mb, in MB), and fall back to
## MEMORY_STATIC only when that is unavailable (e.g. -1 on an unsupported host).
static func _process_memory_bytes() -> int:
	if is_instance_valid(Global.scene_runner):
		var mb: int = Global.scene_runner.get_process_memory_mb()
		if mb > 0:
			return mb * 1024 * 1024
	return int(Performance.get_monitor(Performance.MEMORY_STATIC))
