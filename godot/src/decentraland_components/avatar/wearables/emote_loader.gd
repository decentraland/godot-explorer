class_name EmoteLoader
extends RefCounted

## Helper class for signal-based batched emote loading.
## Connects to ContentProvider signals and manages threaded resource loading.
## Emotes have a special structure with animations that need extraction.

signal all_loads_complete

# Tracks pending loads: file_hash -> true
var _pending_loads: Dictionary = {}
# Tracks completed loads: file_hash -> scene_path
var _completed_loads: Dictionary = {}
# Tracks audio promises (still use promises)
var _audio_promises: Array = []
# Guard against concurrent load operations
var _is_loading: bool = false


func _init():
	Global.content_provider.emote_gltf_ready.connect(_on_emote_ready)
	Global.content_provider.emote_gltf_error.connect(_on_emote_error)


## Cleanup signal connections. Call this when the loader is no longer needed.
func cleanup():
	if Global.content_provider:
		if Global.content_provider.emote_gltf_ready.is_connected(_on_emote_ready):
			Global.content_provider.emote_gltf_ready.disconnect(_on_emote_ready)
		if Global.content_provider.emote_gltf_error.is_connected(_on_emote_error):
			Global.content_provider.emote_gltf_error.disconnect(_on_emote_error)


## Load a single emote using signal-based loading.
## Returns the scene_path if successful, empty string on failure.
func async_load_emote(emote_urn: String, body_shape_id: String) -> String:
	# Wait for any in-progress load to complete before starting a new one
	if _is_loading:
		await all_loads_complete

	_is_loading = true
	_pending_loads.clear()
	_completed_loads.clear()
	_audio_promises.clear()

	var emote = Global.content_provider.get_wearable(emote_urn)
	if emote == null:
		printerr("EmoteLoader: emote ", emote_urn, " is null")
		_is_loading = false
		return ""

	var file_name = emote.get_representation_main_file(body_shape_id)
	if file_name.is_empty():
		printerr(
			"EmoteLoader: no representation for ", emote_urn, " with body shape ", body_shape_id
		)
		_is_loading = false
		return ""

	var content_mapping = emote.get_content_mapping()
	var file_hash = content_mapping.get_hash(file_name)
	if file_hash.is_empty():
		printerr("EmoteLoader: empty file_hash for ", emote_urn)
		_is_loading = false
		return ""

	# Load audio files via promises
	for audio_file in content_mapping.get_files():
		if audio_file.ends_with(".mp3") or audio_file.ends_with(".ogg"):
			var audio_promise = Global.content_provider.fetch_audio(audio_file, content_mapping)
			_audio_promises.push_back(audio_promise)
			break

	# Check if already cached on disk
	if Global.content_provider.is_emote_cached(file_hash):
		var scene_path = Global.content_provider.get_emote_cache_path(file_hash)
		_completed_loads[file_hash] = scene_path

		# Still wait for audio
		if not _audio_promises.is_empty():
			await PromiseUtils.async_all(_audio_promises)

		_is_loading = false
		return scene_path

	# Check if already loading
	if Global.content_provider.is_emote_loading(file_hash):
		_pending_loads[file_hash] = true
	else:
		# Start loading
		Global.content_provider.load_emote_gltf(file_name, content_mapping)
		_pending_loads[file_hash] = true

	# Wait for GLTF load
	if not _pending_loads.is_empty():
		await all_loads_complete

	# Wait for audio
	if not _audio_promises.is_empty():
		await PromiseUtils.async_all(_audio_promises)

	_is_loading = false
	return _completed_loads.get(file_hash, "")


## Load a scene emote (from SDK scene, not wearable).
## Returns the scene_path if successful, empty string on failure.
func async_load_scene_emote(glb_hash: String, base_url: String) -> String:
	# Wait for any in-progress load to complete before starting a new one
	if _is_loading:
		await all_loads_complete

	_is_loading = true
	_pending_loads.clear()
	_completed_loads.clear()

	# Check if already cached on disk
	if Global.content_provider.is_emote_cached(glb_hash):
		_is_loading = false
		return Global.content_provider.get_emote_cache_path(glb_hash)

	# Check if already loading
	if Global.content_provider.is_emote_loading(glb_hash):
		_pending_loads[glb_hash] = true
	else:
		# Create content mapping for the scene emote
		var content_mapping = DclContentMappingAndUrl.from_values(
			base_url + "contents/", {"emote.glb": glb_hash}
		)
		Global.content_provider.load_emote_gltf("emote.glb", content_mapping)
		_pending_loads[glb_hash] = true

	# Wait for load
	if not _pending_loads.is_empty():
		await all_loads_complete

	_is_loading = false
	return _completed_loads.get(glb_hash, "")


## Get a DclEmoteGltf from cached scene using threaded loading.
## Uses ContentProvider's extract_emote_from_scene which extracts animations properly.
func async_get_emote_gltf(file_hash: String) -> DclEmoteGltf:
	var scene_path = _completed_loads.get(file_hash, "")
	if scene_path.is_empty():
		scene_path = Global.content_provider.get_emote_cache_path(file_hash)

	if scene_path.is_empty():
		printerr("EmoteLoader: no scene_path for hash ", file_hash)
		return null

	if not FileAccess.file_exists(scene_path):
		printerr("EmoteLoader: scene file does not exist: ", scene_path)
		return null

	# Use threaded loading for non-blocking
	var err = ResourceLoader.load_threaded_request(scene_path)
	if err != OK:
		printerr("EmoteLoader: failed to request threaded load for ", scene_path)
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
			printerr("EmoteLoader: threaded load FAILED for ", scene_path)
		elif status == ResourceLoader.THREAD_LOAD_INVALID_RESOURCE:
			printerr("EmoteLoader: invalid resource at ", scene_path)
		else:
			printerr("EmoteLoader: unexpected load status ", status, " for ", scene_path)
		return null

	var packed_scene = ResourceLoader.load_threaded_get(scene_path)
	if packed_scene == null:
		printerr("EmoteLoader: loaded resource is null for ", scene_path)
		return null

	if not packed_scene is PackedScene:
		printerr("EmoteLoader: loaded resource is not a PackedScene: ", scene_path)
		return null

	# Use ContentProvider's extract_emote_from_scene to extract animations from loaded scene
	return Global.content_provider.extract_emote_from_scene(packed_scene, file_hash)


func _on_emote_ready(file_hash: String, scene_path: String):
	if _pending_loads.has(file_hash):
		_pending_loads.erase(file_hash)
		_completed_loads[file_hash] = scene_path
		_check_all_complete()


func _on_emote_error(file_hash: String, error: String):
	printerr("EmoteLoader: load error for ", file_hash, ": ", error)
	if _pending_loads.has(file_hash):
		_pending_loads.erase(file_hash)
		_check_all_complete()


func _check_all_complete():
	if _pending_loads.is_empty():
		all_loads_complete.emit()
