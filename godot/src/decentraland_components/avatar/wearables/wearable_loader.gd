class_name WearableLoader
extends RefCounted

## Helper class for promise-based batched wearable loading.
## Uses ContentProvider's promise-based API for GLTF loading.

# Tracks completed loads: file_hash -> scene_path
var _completed_loads: Dictionary = {}


## Load multiple wearables using promise-based loading.
## Returns Dictionary mapping file_hash -> scene_path
func async_load_wearables(wearable_keys: Array, body_shape_id: String) -> Dictionary:
	_completed_loads.clear()

	var gltf_promises: Array = []
	var gltf_file_hashes: Array = []
	var texture_promises: Array = []

	for wearable_key in wearable_keys:
		var wearable = Global.content_provider.get_wearable(wearable_key)
		if wearable == null:
			printerr("WearableLoader: wearable ", wearable_key, " is null")
			continue

		if not DclItemEntityDefinition.is_valid_wearable(wearable, body_shape_id, true):
			continue

		var category = wearable.get_category()

		# Texture wearables (eyes, eyebrows, mouth) use texture loading
		if Wearables.is_texture(category):
			var texture_hashes = Wearables.get_wearable_facial_hashes(wearable, body_shape_id)
			var content_mapping = wearable.get_content_mapping()
			if content_mapping == null:
				printerr("WearableLoader: null content_mapping for texture ", wearable_key)
				continue
			for file_name in content_mapping.get_files():
				for file_hash in texture_hashes:
					if content_mapping.get_hash(file_name) == file_hash:
						var promise = Global.content_provider.fetch_texture(
							file_name, content_mapping
						)
						texture_promises.push_back(promise)
			continue

		# GLTF wearables use promise-based loading
		var file_hash = Wearables.get_item_main_file_hash(wearable, body_shape_id)
		if file_hash.is_empty():
			printerr("WearableLoader: empty file_hash for ", wearable_key)
			continue

		# Start loading - ContentProvider handles caching and deduplication
		var content_mapping = wearable.get_content_mapping()
		if content_mapping == null:
			printerr("WearableLoader: null content_mapping for ", wearable_key)
			continue
		var main_file = wearable.get_representation_main_file(body_shape_id)
		var promise = Global.content_provider.load_wearable_gltf(main_file, content_mapping)
		if promise != null:
			gltf_promises.push_back(promise)
			gltf_file_hashes.push_back(file_hash)

	# Wait for all GLTF promises
	if not gltf_promises.is_empty():
		await PromiseUtils.async_all(gltf_promises)

	# Collect GLTF results
	for i in range(gltf_promises.size()):
		var promise = gltf_promises[i]
		var file_hash = gltf_file_hashes[i]
		if promise.is_resolved() and not promise.is_rejected():
			var scene_path = promise.get_data()
			if scene_path is String and not scene_path.is_empty():
				_completed_loads[file_hash] = scene_path

	# Wait for all texture promises
	if not texture_promises.is_empty():
		await PromiseUtils.async_all(texture_promises)

	return _completed_loads


## Get a wearable node from cached scene, using threaded loading.
## Handles both optimized assets (res:// paths) and runtime-processed assets (user:// paths).
func async_get_wearable_node(file_hash: String) -> Node3D:
	var scene_path = _completed_loads.get(file_hash, "")
	if scene_path.is_empty():
		scene_path = Global.content_provider.get_wearable_cache_path(file_hash)

	if scene_path.is_empty():
		printerr("WearableLoader: no scene_path for hash ", file_hash)
		return null

	# Check if scene exists - use appropriate method for path type
	if scene_path.begins_with("res://"):
		# Optimized asset loaded via resource pack - use ResourceLoader.exists()
		if not ResourceLoader.exists(scene_path):
			printerr("WearableLoader: optimized scene not found: ", scene_path)
			return null
	else:
		# Runtime-processed asset on disk - use FileAccess.file_exists()
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
