class_name ContentManager
extends Node

enum ContentType {
	CT_GLTF_GLB = 1,
	CT_TEXTURE = 2,
	CT_WEARABLE_EMOTE = 3,
	CT_MESHES_MATERIAL = 4,
	CT_INSTACE_GLTF = 5,
	CT_AUDIO = 6,
	CT_VIDEO = 7,
}

const MAX_THREADS = 1

var use_thread = true

var content_threads: Array[ContentThread] = []
var http_requester: RustHttpQueueRequester

var request_monotonic_counter: int = 0

# shared memory...
var content_cache_map: Dictionary = {}


func get_best_content_thread() -> ContentThread:
	var best_content_thread: ContentThread = content_threads[0]
	for i in range(1, content_threads.size()):
		var content_thread: ContentThread = content_threads[i]
		if (
			best_content_thread.content_processing_count()
			> content_thread.content_processing_count()
		):
			best_content_thread = content_thread

	return best_content_thread


func _ready():
	http_requester = RustHttpQueueRequester.new()

	var custom_importer = load("res://src/logic/custom_gltf_importer.gd").new()
	GLTFDocument.register_gltf_document_extension(custom_importer)

	# We do not use threads for tests, running the test in a docker introduces an issue with multithreading on nodes
	# More info: https://github.com/godotengine/godot/issues/79194
	if Global.testing_scene_mode:
		use_thread = false

	if use_thread:
		self.process_mode = Node.PROCESS_MODE_DISABLED
		for id in range(MAX_THREADS):
			var thread = Thread.new()
			var content_thread = ContentThread.new(id + 1, thread)  # id=0 reserved for main thread
			thread.start(self.process_thread.bind(content_thread))
			content_threads.push_back(content_thread)
	else:
		# Only one thread...
		var content_thread = ContentThread.new(0, null)
		content_threads.push_back(content_thread)

	DirAccess.copy_absolute("res://decentraland_logo.png", "user://decentraland_logo.png")


func get_resource_from_hash(file_hash: String):
	var content_cached = content_cache_map.get(file_hash)
	if content_cached != null and content_cached.get("loaded"):
		return content_cached.get("resource")
	return null


func is_resource_from_hash_loaded(file_hash: String):
	var content_cached = content_cache_map.get(file_hash)
	if content_cached != null:
		return content_cached.get("loaded")
	return false


func get_wearable(id: String):
	var wearable_cached = content_cache_map.get(id.to_lower())
	if wearable_cached != null and wearable_cached.get("loaded"):
		return wearable_cached.get("data")
	return null


func duplicate_materials(target_meshes: Array[Dictionary]) -> Promise:
	var promise = Promise.new()

	(
		get_best_content_thread()
		. append_content(
			{
				"content_type": ContentType.CT_MESHES_MATERIAL,
				"target_meshes": target_meshes,
				"promise": promise,
			}
		)
	)

	return promise


func instance_gltf_colliders(
	gltf_node: Node,
	dcl_visible_cmask: int,
	dcl_invisible_cmask: int,
	dcl_scene_id: int,
	dcl_entity_id: int
) -> Promise:
	var promise = Promise.new()
	(
		get_best_content_thread()
		. append_content(
			{
				"content_type": ContentType.CT_INSTACE_GLTF,
				"gltf_node": gltf_node,
				"dcl_visible_cmask": dcl_visible_cmask,
				"dcl_invisible_cmask": dcl_invisible_cmask,
				"dcl_scene_id": dcl_scene_id,
				"dcl_entity_id": dcl_entity_id,
				"promise": promise,
			}
		)
	)

	return promise


# Public function
# @returns $id if the resource was added to queue to fetch, -1 if it had already been fetched
func fetch_wearables(wearables: PackedStringArray, content_base_url: String) -> Promise:
	var new_wearables: PackedStringArray = []
	var new_id: int = request_monotonic_counter + 1

	var last_wearable_promise: Promise = null
	var promise = Promise.new()

	for wearable in wearables:
		var wearable_lower = wearable.to_lower()
		var wearable_cached = content_cache_map.get(wearable_lower)
		if wearable_cached == null:
			content_cache_map[wearable_lower] = {
				"id": new_id,
				"loaded": false,
				"promise": promise,
			}
			new_wearables.append(wearable_lower)
		else:
			last_wearable_promise = wearable_cached["promise"]

	if new_wearables.is_empty():
		return last_wearable_promise

	request_monotonic_counter = new_id
	(
		get_best_content_thread()
		. append_content(
			{
				"id": new_id,
				"content_type": ContentType.CT_WEARABLE_EMOTE,
				"new_wearables": new_wearables,
				"content_base_url": content_base_url,
				"promise": promise,
			}
		)
	)

	return promise


# Public function
# @returns request state on success, null if it had already been fetched
func fetch_gltf(file_path: String, content_mapping: Dictionary) -> Promise:
	var promise = Promise.new()
	var file_hash: String = content_mapping.get("content", {}).get(file_path, "")
	var content_cached = content_cache_map.get(file_hash)
	if content_cached != null:
		return content_cached.get("promise")

	content_cache_map[file_hash] = {"loaded": false, "promise": promise}

	(
		get_best_content_thread()
		. append_content(
			{
				"file_path": file_path,
				"file_hash": file_hash,
				"content_type": ContentType.CT_GLTF_GLB,
				"content_mapping": content_mapping,
			}
		)
	)

	return promise


# Public function
# @returns true if the resource was added to queue to fetch, false if it had already been fetched
func fetch_texture(file_path: String, content_mapping: Dictionary) -> Promise:
	var file_hash: String = content_mapping.get("content", {}).get(file_path, "")
	return fetch_texture_by_hash(file_hash, content_mapping)


func fetch_texture_by_hash(file_hash: String, content_mapping: Dictionary):
	var url = content_mapping.get("base_url") + file_hash
	return fetch_texture_by_url(file_hash, url)


func fetch_texture_by_url(file_hash: String, url: String):
	var promise = Promise.new()
	var content_cached = content_cache_map.get(file_hash)
	if content_cached != null:
		return content_cached.get("promise")

	content_cache_map[file_hash] = {"loaded": false, "promise": promise}

	(
		get_best_content_thread()
		. append_content(
			{
				"file_hash": file_hash,
				"url": url,
				"content_type": ContentType.CT_TEXTURE,
			}
		)
	)

	return promise


func get_image_from_texture_or_null(file_path: String, content_mapping: Dictionary) -> Image:
	var file_hash: String = content_mapping.get("content", {}).get(file_path, "")
	return get_image_from_texture_by_hash_or_null(file_hash)


func get_image_from_texture_by_hash_or_null(file_hash: String) -> Image:
	var content_cached = content_cache_map.get(file_hash)
	if content_cached != null:
		return content_cached.get("image")
	return null


func fetch_audio(file_path: String, content_mapping: Dictionary) -> Promise:
	var promise = Promise.new()
	var file_hash: String = content_mapping.get("content", {}).get(file_path, "")
	var content_cached = content_cache_map.get(file_hash)
	if content_cached != null:
		return content_cached.get("promise")

	content_cache_map[file_hash] = {"loaded": false, "promise": promise}

	(
		get_best_content_thread()
		. append_content(
			{
				"file_path": file_path,
				"file_hash": file_hash,
				"content_type": ContentType.CT_AUDIO,
				"content_mapping": content_mapping,
			}
		)
	)

	return promise


# Public function
# @returns true if the resource was added to queue to fetch, false if it had already been fetched
func fetch_video(file_hash: String, content_mapping: Dictionary) -> Promise:
	var promise = Promise.new()
	var content_cached = content_cache_map.get(file_hash)
	if content_cached != null:
		return content_cached.get("promise")

	content_cache_map[file_hash] = {"loaded": false, "promise": promise}

	(
		get_best_content_thread()
		. append_content(
			{
				"content_mapping": content_mapping,
				"file_hash": file_hash,
				"content_type": ContentType.CT_VIDEO,
			}
		)
	)

	return promise


# Should be disabled on single thread...
func _process(_dt: float) -> void:
	# Main thread
	if use_thread == false:
		content_threads[0].process(content_cache_map)


func process_thread(content_thread: ContentThread):
	while true:
		content_thread.process(content_cache_map)
		OS.delay_msec(1)


func split_animations(_gltf_node: Node) -> void:
	pass


#	# TODO: multiple animations
#	var animation_player: AnimationPlayer = gltf_node.get_node("AnimationPlayer")
#	if animation_player == null:
#		return
#
#	var index: int = 0
#	var animation_players = []
#	var anims := animation_player.get_animation_list()
#	for current_anim in anims:
#		var dedicated_anim_player = animation_player.duplicate()
#		dedicated_anim_player.set_name("AnimationPlayer_" + str(index))
#		dedicated_anim_player.set_meta("anim_name", current_anim)
#		gltf_node.add_child(dedicated_anim_player)
#		index += 1
#
#	gltf_node.remove_child(animation_player)


# TODO(Mateo): Looks like more a helper than part of the ContentThreadPool
func hide_colliders(gltf_node):
	for maybe_collider in gltf_node.get_children():
		if maybe_collider is Node3D and maybe_collider.name.find("_collider") != -1:
			maybe_collider.visible = false

		if maybe_collider is Node:
			hide_colliders(maybe_collider)
