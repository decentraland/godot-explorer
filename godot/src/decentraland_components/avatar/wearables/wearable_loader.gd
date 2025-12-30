class_name WearableLoader
extends RefCounted

## Helper class for signal-based batched wearable loading.
## Connects to ContentProvider signals and manages threaded resource loading.

signal all_loads_complete

# Tracks pending loads: file_hash -> true
var _pending_loads: Dictionary = {}
# Tracks completed loads: file_hash -> scene_path
var _completed_loads: Dictionary = {}
# Tracks texture loads (still use promises)
var _texture_promises: Array = []
# Guard against concurrent load operations
var _is_loading: bool = false


func _init():
	Global.content_provider.wearable_gltf_ready.connect(_on_wearable_ready)
	Global.content_provider.wearable_gltf_error.connect(_on_wearable_error)


## Cleanup signal connections. Call this when the loader is no longer needed.
func cleanup():
	if Global.content_provider:
		if Global.content_provider.wearable_gltf_ready.is_connected(_on_wearable_ready):
			Global.content_provider.wearable_gltf_ready.disconnect(_on_wearable_ready)
		if Global.content_provider.wearable_gltf_error.is_connected(_on_wearable_error):
			Global.content_provider.wearable_gltf_error.disconnect(_on_wearable_error)


## Load multiple wearables using signal-based loading.
## Returns Dictionary mapping file_hash -> scene_path
func async_load_wearables(wearable_keys: Array, body_shape_id: String) -> Dictionary:
	# Wait for any in-progress load to complete before starting a new one
	if _is_loading:
		await all_loads_complete

	_is_loading = true
	_pending_loads.clear()
	_completed_loads.clear()
	_texture_promises.clear()

	for wearable_key in wearable_keys:
		var wearable = Global.content_provider.get_wearable(wearable_key)
		if wearable == null:
			printerr("WearableLoader: wearable ", wearable_key, " is null")
			continue

		if not DclItemEntityDefinition.is_valid_wearable(wearable, body_shape_id, true):
			continue

		var category = wearable.get_category()

		# Texture wearables (eyes, eyebrows, mouth) still use promise-based loading
		if Wearables.is_texture(category):
			var texture_hashes = Wearables.get_wearable_facial_hashes(wearable, body_shape_id)
			var content_mapping = wearable.get_content_mapping()
			for file_name in content_mapping.get_files():
				for file_hash in texture_hashes:
					if content_mapping.get_hash(file_name) == file_hash:
						var promise = Global.content_provider.fetch_texture(
							file_name, content_mapping
						)
						_texture_promises.push_back(promise)
			continue

		# GLTF wearables use signal-based loading
		var file_hash = Wearables.get_item_main_file_hash(wearable, body_shape_id)
		if file_hash.is_empty():
			printerr("WearableLoader: empty file_hash for ", wearable_key)
			continue

		# Check if already cached on disk
		if Global.content_provider.is_wearable_cached(file_hash):
			var scene_path = Global.content_provider.get_wearable_cache_path(file_hash)
			_completed_loads[file_hash] = scene_path
			continue

		# Check if already loading
		if Global.content_provider.is_wearable_loading(file_hash):
			_pending_loads[file_hash] = true
			continue

		# Start loading
		var content_mapping = wearable.get_content_mapping()
		var main_file = wearable.get_representation_main_file(body_shape_id)
		Global.content_provider.load_wearable_gltf(main_file, content_mapping)
		_pending_loads[file_hash] = true

	# Wait for all pending GLTF loads
	if not _pending_loads.is_empty():
		await all_loads_complete

	# Wait for all texture promises
	if not _texture_promises.is_empty():
		await PromiseUtils.async_all(_texture_promises)

	_is_loading = false
	return _completed_loads


## Get a wearable node from cached scene, using threaded loading.
func async_get_wearable_node(file_hash: String) -> Node3D:
	var scene_path = _completed_loads.get(file_hash, "")
	if scene_path.is_empty():
		scene_path = Global.content_provider.get_wearable_cache_path(file_hash)

	if scene_path.is_empty():
		printerr("WearableLoader: no scene_path for hash ", file_hash)
		return null

	if not FileAccess.file_exists(scene_path):
		printerr("WearableLoader: scene file does not exist: ", scene_path)
		return null

	# Use threaded loading for non-blocking
	var err = ResourceLoader.load_threaded_request(scene_path)
	if err != OK:
		printerr("WearableLoader: failed to request threaded load for ", scene_path)
		return null

	# Poll until loaded
	var main_tree = Engine.get_main_loop()
	var status = ResourceLoader.load_threaded_get_status(scene_path)
	while status == ResourceLoader.THREAD_LOAD_IN_PROGRESS:
		if not is_instance_valid(main_tree):
			return null
		await main_tree.process_frame
		status = ResourceLoader.load_threaded_get_status(scene_path)

	if status != ResourceLoader.THREAD_LOAD_LOADED:
		if status == ResourceLoader.THREAD_LOAD_FAILED:
			printerr("WearableLoader: threaded load FAILED for ", scene_path)
		elif status == ResourceLoader.THREAD_LOAD_INVALID_RESOURCE:
			printerr("WearableLoader: invalid resource at ", scene_path)
		else:
			printerr("WearableLoader: unexpected load status ", status, " for ", scene_path)
		return null

	var packed_scene = ResourceLoader.load_threaded_get(scene_path)
	if packed_scene == null:
		printerr("WearableLoader: loaded resource is null for ", scene_path)
		return null

	if not packed_scene is PackedScene:
		printerr("WearableLoader: loaded resource is not a PackedScene: ", scene_path)
		return null

	return packed_scene.instantiate()


func _on_wearable_ready(file_hash: String, scene_path: String):
	if _pending_loads.has(file_hash):
		_pending_loads.erase(file_hash)
		_completed_loads[file_hash] = scene_path
		_check_all_complete()


func _on_wearable_error(file_hash: String, error: String):
	printerr("WearableLoader: load error for ", file_hash, ": ", error)
	if _pending_loads.has(file_hash):
		_pending_loads.erase(file_hash)
		_check_all_complete()


func _check_all_complete():
	if _pending_loads.is_empty():
		all_loads_complete.emit()
