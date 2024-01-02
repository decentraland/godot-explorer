class_name ContentThread
extends RefCounted

enum ContentType {
	CT_GLTF_GLB = 1,
	CT_TEXTURE = 2,
	CT_WEARABLE_EMOTE = 3,
	CT_MESHES_MATERIAL = 4,
	CT_INSTACE_GLTF = 5,
	CT_AUDIO = 6,
	CT_VIDEO = 7
}

# Public
var thread: Thread = null  # null=main_thread
var id: int = -1

# Private
var _pending_content: Array[Dictionary] = []
var _http_requester: RustHttpRequesterWrapper

# Metrics
var _processing_count = 0


func append_content(request: Dictionary):
	_pending_content.push_back(request)


func content_processing_count():
	return _pending_content.size()


func _init(param_id: int, param_thread: Thread):
	_http_requester = RustHttpRequesterWrapper.new()
	self.thread = param_thread
	self.id = param_id


func process(content_cache_map: Dictionary):  # not a coroutine
	_http_requester.poll()

	if _pending_content.is_empty():
		return

	var loading_content: Array[Dictionary] = []
	while _pending_content.size() > 0:
		loading_content.push_back(_pending_content.pop_front())

	for content in loading_content:
		# coroutine not awaited, it can process multiple times
		async_process_content(content, content_cache_map)


# coroutine
func async_process_content(content: Dictionary, content_cache_map: Dictionary):
	_processing_count += 1
	var content_type: ContentType = content.get("content_type")
	match content_type:
		ContentType.CT_GLTF_GLB:
			await _async_process_loading_gltf(content, content_cache_map)

		ContentType.CT_TEXTURE:
			await _async_process_loading_texture(content, content_cache_map)

		ContentType.CT_AUDIO:
			await _async_process_loading_audio(content, content_cache_map)

		ContentType.CT_WEARABLE_EMOTE:
			await _async_process_loading_wearable(content, content_cache_map)

		ContentType.CT_MESHES_MATERIAL:
			_process_meshes_material(content)

		ContentType.CT_INSTACE_GLTF:
			_process_instance_gltf(content)

		ContentType.CT_VIDEO:
			await _async_process_loading_video(content, content_cache_map)

		_:
			printerr("Fetching invalid content type ", content_type)

	_processing_count -= 1


func _process_meshes_material(content: Dictionary):
	var target_meshes: Array[Dictionary] = content.get("target_meshes")

	for mesh_dict in target_meshes:
		var mesh = mesh_dict.get("mesh")
		for i in range(mesh_dict.get("n")):
			var material = mesh.surface_get_material(i).duplicate(true)
			mesh.surface_set_material(i, material)

	var promise: Promise = content["promise"]
	promise.call_deferred("resolve")


func _async_process_loading_wearable(
	content: Dictionary,
	content_cache_map: Dictionary,
) -> void:
	var url: String = (
		content.get("content_base_url", "https://peer.decentraland.org/content") + "entities/active"
	)
	var wearables: PackedStringArray = content.get("new_wearables", [])
	if wearables.is_empty():
		printerr("Trying to fetch empty wearables")
		return

	var json_payload: String = JSON.stringify({"pointers": wearables})
	var headers = ["Content-Type: application/json"]

	var promise: Promise = _http_requester.request_json(
		url, HTTPClient.METHOD_POST, json_payload, headers
	)

	var content_result = await PromiseUtils.async_awaiter(promise)
	if content_result is PromiseError:
		printerr("Failing on loading wearable ", url, " reason: ", content_result.get_error())
		return

	var pointers_missing: Array = content["new_wearables"]
	var pointer_fetched: Array = []

	var response = content_result.get_string_response_as_json()
	if not response is Array:
		# TODO: clean cached?
		return

	for item in response:
		if not item is Dictionary:
			# TODO: clean cached?
			continue

		var pointers: Array = item.get("pointers", [])
		for pointer in pointers:
			var lower_pointer_fetched = pointer.to_lower()
			if pointers_missing.find(lower_pointer_fetched) != -1:
				content_cache_map[lower_pointer_fetched]["data"] = item
				content_cache_map[lower_pointer_fetched]["loaded"] = true
				pointer_fetched.push_back(lower_pointer_fetched)

		var wearable_content_dict: Dictionary = {}
		var wearable_content: Array = item.get("content", [])
		for content_item in wearable_content:
			wearable_content_dict[content_item.file.to_lower()] = content_item.hash
		item["content"] = wearable_content_dict

	for pointer in pointer_fetched:
		pointers_missing.erase(pointer)

	if not pointers_missing.is_empty():
		for pointer in pointers_missing:
			printerr("Missing pointer ", pointer)
			content_cache_map[pointer]["loaded"] = true
			content_cache_map[pointer]["data"] = null

	var result_promise: Promise = content["promise"]
	result_promise.call_deferred("resolve")


func _get_gltf_dependencies(local_gltf_path: String) -> Array[String]:
	var dependencies: Array[String] = []
	var p_file := FileAccess.open(local_gltf_path, FileAccess.READ)
	p_file.seek(0)

	var magic := p_file.get_32()
	var text: String
	if magic == 0x46546C67:
		p_file.get_32()  # version
		p_file.get_32()  # length
		var chunk_length := p_file.get_32()
		var chunk_type := p_file.get_32()
		var json_data := p_file.get_buffer(chunk_length)
		text = json_data.get_string_from_utf8()
	else:
		p_file.seek(0)
		text = p_file.get_as_utf8_string()

	var json = JSON.parse_string(text)
	if json == null:
		printerr("Failing on loading gltf when parsing the JSON")
		return dependencies

	for image in json.get("images", []):
		var uri = image.get("uri", "")
		if not uri.is_empty() and not uri.begins_with("data:"):
			dependencies.push_back(String(uri))
	for buf in json.get("buffers", []):
		var uri = buf.get("uri", "")
		if not uri.is_empty() and not uri.begins_with("data:"):
			dependencies.push_back(String(uri))

	return dependencies


func _async_process_loading_gltf(content: Dictionary, content_cache_map: Dictionary) -> void:
	var content_mapping = content.get("content_mapping")
	var file_hash: String = content.get("file_hash")
	var file_path: String = content.get("file_path")
	var base_url: String = content_mapping.get("base_url", "")
	var base_path = file_path.get_base_dir()
	var local_gltf_path = "user://content/" + file_hash

	if file_hash.is_empty() or base_url.is_empty():
		printerr("hash or base_url is empty")
		return
	var file_hash_path = base_url + file_hash

	# If gltf doesn't exists locally, we request it
	if !FileAccess.file_exists(local_gltf_path):
		var absolute_file_path = local_gltf_path.replace("user:/", OS.get_user_data_dir())
		var request_promise = _http_requester.request_file(file_hash_path, absolute_file_path)
		var content_result = await PromiseUtils.async_awaiter(request_promise)
		if content_result is PromiseError:
			printerr(
				"Failing on loading gltf ", file_hash_path, " reason: ", content_result.get_error()
			)
			return

	# Load gltf dependencies
	var mappings: Dictionary = {}
	var promises_dependencies: Array[Promise] = []
	var dependencies = _get_gltf_dependencies(local_gltf_path)

	for uri in dependencies:
		var image_path
		if base_path.is_empty():
			image_path = uri
		else:
			image_path = base_path + "/" + uri
		var image_hash = content_mapping.get("content", {}).get(image_path.to_lower(), "")
		if image_hash.is_empty() or base_url.is_empty():
			printerr(uri + " not found (resolved: " + image_path + ") => ", content_mapping)
			continue

		var local_image_path = "user://content/" + image_hash
		if not FileAccess.file_exists(local_image_path):
			var absolute_file_path = local_image_path.replace("user:/", OS.get_user_data_dir())
			promises_dependencies.push_back(
				_http_requester.request_file(base_url + image_hash, absolute_file_path)
			)
		mappings[uri] = "content/" + image_hash

	content["gltf_mappings"] = mappings

	await PromiseUtils.async_all(promises_dependencies)

	# final processing
	var new_gltf := GLTFDocument.new()
	var new_gltf_state := GLTFState.new()

	new_gltf_state.set_additional_data("base_path", base_path)
	new_gltf_state.set_additional_data("mappings", content["gltf_mappings"])
	var err = new_gltf.append_from_file(local_gltf_path, new_gltf_state, 0, OS.get_user_data_dir())

	var node = new_gltf.generate_scene(new_gltf_state)
	if node != null:
		node.rotate_y(PI)
		create_colliders(node)
		if err != OK:
			push_warning("resource with errors ", file_path, " : ", err)
	else:
		printerr("resource resolved as null ", file_path, " err?", err)

	content_cache_map[file_hash]["resource"] = node
	content_cache_map[file_hash]["loaded"] = true
	var promise: Promise = content_cache_map[file_hash].get("promise", null)
	promise.call_deferred("resolve")


func _async_process_loading_texture(
	content: Dictionary,
	content_cache_map: Dictionary,
) -> void:
	var file_hash: String = content.get("file_hash")
	var url: String = content.get("url", "")
	var local_texture_path = "user://content/" + file_hash
	if file_hash.is_empty() or url.is_empty():
		content_cache_map[file_hash]["promise"].call_deferred("reject", "Hash or url is empty")
		return

	# with testing_scene_mode we don't cache textures TODO: see if this mode could be more generic e.g. no_cache
	if !FileAccess.file_exists(local_texture_path) or Global.testing_scene_mode:
		var absolute_file_path = local_texture_path.replace("user:/", OS.get_user_data_dir())

		var promise_texture_file: Promise = _http_requester.request_file(url, absolute_file_path)

		var content_result = await PromiseUtils.async_awaiter(promise_texture_file)
		if content_result is PromiseError:
			content_cache_map[file_hash]["promise"].call_deferred(
				"reject",
				"Failing on loading texture " + url + " reason: " + str(content_result.get_error())
			)
			return

	var file = FileAccess.open(local_texture_path, FileAccess.READ)
	if file == null:
		content_cache_map[file_hash]["promise"].call_deferred("reject", "texture download fails")
		return

	var buf = file.get_buffer(file.get_length())
	var image := Image.new()
	var err = image.load_png_from_buffer(buf)
	if err != OK:
		content_cache_map[file_hash]["promise"].call_deferred(
			"reject", "Texture  " + url + " couldn't be loaded succesfully: " + str(err)
		)
		return

	var content_cache = content_cache_map[file_hash]
	var resource = ImageTexture.create_from_image(image)
	content_cache["image"] = image
	content_cache["resource"] = resource
	content_cache["loaded"] = true

	var promise: Promise = content_cache["promise"]
	promise.call_deferred("resolve_with_data", resource)


func _async_process_loading_audio(
	content: Dictionary,
	content_cache_map: Dictionary,
) -> void:
	var content_mapping = content.get("content_mapping")
	var file_hash: String = content.get("file_hash")
	var base_url: String = content_mapping.get("base_url", "")
	var local_audio_path = "user://content/" + file_hash

	if file_hash.is_empty() or base_url.is_empty():
		printerr("hash or base_url is empty")
		return
	var file_hash_path = base_url + file_hash

	if !FileAccess.file_exists(local_audio_path):
		var absolute_file_path = local_audio_path.replace("user:/", OS.get_user_data_dir())

		var promise_audio_file: Promise = _http_requester.request_file(
			file_hash_path, absolute_file_path
		)
		var content_result = await PromiseUtils.async_awaiter(promise_audio_file)
		if content_result is PromiseError:
			printerr(
				"Failing on loading wearable ",
				file_hash_path,
				" reason: ",
				content_result.get_error()
			)
			return

	var file := FileAccess.open(local_audio_path, FileAccess.READ)
	if file == null:
		printerr("audio download fails")
		return

	var file_path: String = content.get("file_path")
	var bytes = file.get_buffer(file.get_length())
	var audio_stream = null

	if file_path.ends_with(".wav"):
		audio_stream = AudioStreamWAV.new()
		audio_stream.data = bytes
	elif file_path.ends_with(".ogg"):
		audio_stream = AudioStreamOggVorbis.new()
		audio_stream.data = bytes
	elif file_path.ends_with(".mp3"):
		audio_stream = AudioStreamMP3.new()
		audio_stream.data = bytes

	if audio_stream == null:
		printerr("Audio " + base_url + file_hash + " unrecognized format (infered by file path)")
		return

	var content_cache = content_cache_map[file_hash]
	content_cache["resource"] = audio_stream
	content_cache["loaded"] = true

	var promise: Promise = content_cache["promise"]
	promise.call_deferred("resolve_with_data", audio_stream)
	return


func _async_process_loading_video(
	content: Dictionary,
	content_cache_map: Dictionary,
) -> void:
	var content_mapping = content.get("content_mapping")
	var file_hash: String = content.get("file_hash")
	var base_url: String = content_mapping.get("base_url", "")
	var local_video_path = "user://content/" + file_hash

	if file_hash.is_empty() or base_url.is_empty():
		printerr("hash or base_url is empty")
		return
	var file_hash_path = base_url + file_hash

	if !FileAccess.file_exists(local_video_path):
		var absolute_file_path = local_video_path.replace("user:/", OS.get_user_data_dir())
		var promise_video_file: Promise = _http_requester.request_file(
			base_url + file_hash, absolute_file_path
		)
		var content_result = await PromiseUtils.async_awaiter(promise_video_file)
		if content_result is PromiseError:
			printerr(
				"Failing on loading wearable ",
				file_hash_path,
				" reason: ",
				content_result.get_error()
			)
			return

	# process texture
	var file := FileAccess.open(local_video_path, FileAccess.READ)
	if file == null:
		printerr("video download fails")
		return

	var content_cache = content_cache_map[file_hash]
	content_cache["loaded"] = true
	var promise: Promise = content_cache["promise"]
	promise.call_deferred("resolve")


func create_colliders(node_to_inspect: Node):
	for node in node_to_inspect.get_children():
		if node is MeshInstance3D:
			var invisible_mesh = node.name.find("_collider") != -1
			var static_body_3d: StaticBody3D = get_collider(node)
			if static_body_3d == null:
				node.create_trimesh_collision()
				static_body_3d = get_collider(node)
				if static_body_3d == null:
					printerr("static_body_3d is null...", node.get_tree())

			if static_body_3d != null:
				static_body_3d.name = node.name + "_colgen"
				var parent = static_body_3d.get_parent()
				var new_animatable = AnimatableBody3D.new()
				parent.add_child(new_animatable)
				parent.remove_child(static_body_3d)

				for child in static_body_3d.get_children(true):
					static_body_3d.remove_child(child)
					new_animatable.add_child(child)
					if child is CollisionShape3D and child.shape is ConcavePolygonShape3D:
						# TODO: workaround, the face's normals probably need to be inverted in some meshes
						child.shape.backface_collision = true

				new_animatable.sync_to_physics = false
				new_animatable.process_mode = Node.PROCESS_MODE_DISABLED

				new_animatable.set_meta("dcl_col", 0)
				new_animatable.collision_layer = 0
				new_animatable.collision_mask = 0

				new_animatable.set_meta("invisible_mesh", invisible_mesh)

			if invisible_mesh:
				node.visible = false

		if node is Node:
			create_colliders(node)


func _process_instance_gltf(content: Dictionary):
	var gltf_node: Node = content.get("gltf_node")
	var dcl_visible_cmask: int = content.get("dcl_visible_cmask")
	var dcl_invisible_cmask: int = content.get("dcl_invisible_cmask")
	var dcl_scene_id: int = content.get("dcl_scene_id")
	var dcl_entity_id: int = content.get("dcl_entity_id")

	gltf_node = gltf_node.duplicate()

	var to_remove_nodes = []
	update_set_mask_colliders(
		gltf_node,
		dcl_visible_cmask,
		dcl_invisible_cmask,
		dcl_scene_id,
		dcl_entity_id,
		to_remove_nodes
	)

	for node in to_remove_nodes:
		node.get_parent().remove_child(node)

	var promise: Promise = content["promise"]
	promise.call_deferred("resolve_with_data", gltf_node)


func get_collider(mesh_instance: MeshInstance3D):
	for maybe_static_body in mesh_instance.get_children():
		if maybe_static_body is StaticBody3D:
			return maybe_static_body
	return null


func update_set_mask_colliders(
	node_to_inspect: Node,
	dcl_visible_cmask: int,
	dcl_invisible_cmask: int,
	dcl_scene_id: int,
	dcl_entity_id: int,
	to_remove_nodes: Array
):
	for node in node_to_inspect.get_children():
		if node is AnimatableBody3D:
			var mask: int = 0
			var invisible_mesh = node.has_meta("invisible_mesh") and node.get_meta("invisible_mesh")
			if invisible_mesh:
				mask = dcl_invisible_cmask
			else:
				mask = dcl_visible_cmask

			var resolved_node = node
			if not node.has_meta("dcl_scene_id"):
				var parent = node.get_parent()
				resolved_node = node.duplicate()
				resolved_node.name = node.name + "_instanced"
				resolved_node.set_meta("dcl_scene_id", dcl_scene_id)
				resolved_node.set_meta("dcl_entity_id", dcl_entity_id)

				parent.add_child(resolved_node)
				to_remove_nodes.push_back(node)

			resolved_node.set_meta("dcl_col", mask)
			resolved_node.collision_layer = mask
			resolved_node.collision_mask = 0
			if mask == 0:
				resolved_node.process_mode = Node.PROCESS_MODE_DISABLED
			else:
				resolved_node.process_mode = Node.PROCESS_MODE_INHERIT

		if node is Node:
			update_set_mask_colliders(
				node,
				dcl_visible_cmask,
				dcl_invisible_cmask,
				dcl_scene_id,
				dcl_entity_id,
				to_remove_nodes
			)
