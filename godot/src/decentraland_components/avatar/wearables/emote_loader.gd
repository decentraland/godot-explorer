class_name EmoteLoader
extends RefCounted

## Helper class for promise-based batched emote loading.
## Emotes have a special structure with animations that need extraction.
## Scene paths are retrieved from ContentProvider's cache rather than tracked locally.


## Load a single emote using promise-based loading.
## Returns the scene_path if successful, empty string on failure.
func async_load_emote(emote_urn: String, body_shape_id: String) -> String:
	var emote = Global.content_provider.get_wearable(emote_urn)
	if emote == null:
		printerr("EmoteLoader: emote ", emote_urn, " is null")
		return ""

	var file_name = emote.get_representation_main_file(body_shape_id)
	if file_name.is_empty():
		printerr(
			"EmoteLoader: no representation for ", emote_urn, " with body shape ", body_shape_id
		)
		return ""

	var content_mapping = emote.get_content_mapping()
	if content_mapping == null:
		printerr("EmoteLoader: null content_mapping for ", emote_urn)
		return ""

	var file_hash = content_mapping.get_hash(file_name)
	if file_hash.is_empty():
		printerr("EmoteLoader: empty file_hash for ", emote_urn)
		return ""

	# Load audio files via promises (in parallel with GLTF)
	var audio_promises: Array = []
	for audio_file in content_mapping.get_files():
		if audio_file.ends_with(".mp3") or audio_file.ends_with(".ogg"):
			var audio_promise = Global.content_provider.fetch_audio(audio_file, content_mapping)
			audio_promises.push_back(audio_promise)
			break

	# Start loading GLTF - ContentProvider handles caching and deduplication
	var gltf_promise = Global.content_provider.load_emote_gltf(file_name, content_mapping)
	if gltf_promise == null:
		printerr("EmoteLoader: failed to start loading emote ", emote_urn)
		return ""

	# Wait for GLTF to load
	await PromiseUtils.async_awaiter(gltf_promise)

	var scene_path = ""
	if gltf_promise.is_resolved() and not gltf_promise.is_rejected():
		var result = gltf_promise.get_data()
		if result is String:
			scene_path = result

	# Wait for audio (if any)
	if not audio_promises.is_empty():
		await PromiseUtils.async_all(audio_promises)

	return scene_path


## Load a scene emote (from SDK scene, not wearable).
## Returns the scene_path if successful, empty string on failure.
func async_load_scene_emote(glb_hash: String, base_url: String) -> String:
	# Create content mapping for the scene emote
	var content_mapping = DclContentMappingAndUrl.from_values(
		base_url + "contents/", {"emote.glb": glb_hash}
	)

	# Start loading - ContentProvider handles caching and deduplication
	var gltf_promise = Global.content_provider.load_emote_gltf("emote.glb", content_mapping)
	if gltf_promise == null:
		printerr("EmoteLoader: failed to start loading scene emote ", glb_hash)
		return ""

	# Wait for load
	await PromiseUtils.async_awaiter(gltf_promise)

	if gltf_promise.is_resolved() and not gltf_promise.is_rejected():
		var result = gltf_promise.get_data()
		if result is String:
			return result

	return ""


## Get a DclEmoteGltf from cached scene using threaded loading.
## Uses ContentProvider's extract_emote_from_scene which extracts animations properly.
func async_get_emote_gltf(file_hash: String) -> DclEmoteGltf:
	# Get scene path from ContentProvider's cache
	var scene_path = Global.content_provider.get_emote_cache_path(file_hash)

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
