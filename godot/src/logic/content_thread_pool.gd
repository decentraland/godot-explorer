extends Node
class_name ContentManager

enum ContentType {
	CT_GLTF_GLB = 1,
	CT_TEXTURE = 2,
	CT_WEARABLE_EMOTE = 3,
	CT_MESHES_MATERIAL = 4,
	CT_INSTACE_GLTF = 5,
	CT_AUDIO = 6,
	CT_VIDEO = 7
}

#var loading_content: Array[Dictionary] = []
var pending_content: Array[Dictionary] = []
var content_cache_map: Dictionary = {}
var content_thread_pool: Thread = null
var http_requester = RustHttpRequesterWrapper.new()
var wearable_cache_map: Dictionary = {}
var request_monotonic_counter: int = 0
var promise_dictionary: Dictionary = {}

var use_thread = true


func _ready():
	var custom_importer = load("res://src/logic/custom_gltf_importer.gd").new()
	GLTFDocument.register_gltf_document_extension(custom_importer)

	if use_thread:
		self.process_mode = Node.PROCESS_MODE_DISABLED
		content_thread_pool = Thread.new()
		content_thread_pool.start(self.content_thread_pool_func)

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
	var wearable_cached = wearable_cache_map.get(id.to_lower())
	if wearable_cached != null and wearable_cached.get("loaded"):
		return wearable_cached.get("data")
	return null


func duplicate_materials(target_meshes: Array[Dictionary]) -> Promise:
	var promise = Promise.new()

	(
		pending_content
		. push_back(
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
		pending_content
		. push_back(
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
		var wearable_cached = wearable_cache_map.get(wearable_lower)
		if wearable_cached == null:
			wearable_cache_map[wearable_lower] = {
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
		pending_content
		. push_back(
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

	pending_content.push_back(
		{
			"file_path": file_path,
			"file_hash": file_hash,
			"content_type": ContentType.CT_GLTF_GLB,
			"content_mapping": content_mapping,
		}
	)

	return promise


# Public function
# @returns true if the resource was added to queue to fetch, false if it had already been fetched
func fetch_texture(file_path: String, content_mapping: Dictionary) -> Promise:
	var file_hash: String = content_mapping.get("content", {}).get(file_path, "")
	return fetch_texture_by_hash(file_hash, content_mapping)


func fetch_texture_by_hash(file_hash: String, content_mapping: Dictionary):
	var promise = Promise.new()
	var content_cached = content_cache_map.get(file_hash)
	if content_cached != null:
		return content_cached.get("promise")

	content_cache_map[file_hash] = {"loaded": false, "promise": promise}

	pending_content.push_back(
		{
			"file_hash": file_hash,
			"content_type": ContentType.CT_TEXTURE,
			"content_mapping": content_mapping,
		}
	)

	return promise


func fetch_audio(file_path: String, content_mapping: Dictionary) -> Promise:
	var promise = Promise.new()
	var file_hash: String = content_mapping.get("content", {}).get(file_path, "")
	var content_cached = content_cache_map.get(file_hash)
	if content_cached != null:
		return content_cached.get("promise")

	content_cache_map[file_hash] = {"loaded": false, "promise": promise}

	pending_content.push_back(
		{
			"file_path": file_path,
			"file_hash": file_hash,
			"content_type": ContentType.CT_AUDIO,
			"content_mapping": content_mapping,
		}
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

	pending_content.push_back(
		{
			"content_mapping": content_mapping,
			"file_hash": file_hash,
			"content_type": ContentType.CT_VIDEO,
		}
	)

	return promise


func _process(_dt: float) -> void:
	_th_poll()

func content_thread_pool_func():
	while true:
		_th_poll()
		OS.delay_msec(1)

func _th_poll():
	http_requester.poll()
	var loading_content: Array[Dictionary] = []
	while pending_content.size() > 0:
		loading_content.push_back(pending_content.pop_front())

	for content in loading_content:
		var content_type: ContentType = content.get("content_type")
		match content_type:
			ContentType.CT_GLTF_GLB:
				await _process_loading_gltf(content)

			ContentType.CT_TEXTURE:
				await _process_loading_texture(content)

			ContentType.CT_AUDIO:
				await _process_loading_audio(content)

			ContentType.CT_WEARABLE_EMOTE:
				await _process_loading_wearable(content)

			ContentType.CT_MESHES_MATERIAL:
				await _process_meshes_material(content)

			ContentType.CT_INSTACE_GLTF:
				await _process_instance_gltf(content)

			ContentType.CT_VIDEO:
				await _process_loading_video(content)

			_:
				printerr("Fetching invalid content type ", content_type)


func _process_meshes_material(content: Dictionary):
	var target_meshes: Array[Dictionary] = content.get("target_meshes")

	for mesh_dict in target_meshes:
		var mesh = mesh_dict.get("mesh")
		for i in range(mesh_dict.get("n")):
			var material = mesh.surface_get_material(i).duplicate(true)
			mesh.surface_set_material(i, material)

	var promise: Promise = content["promise"]
	promise.call_deferred("resolve")
	return false

func _process_loading_wearable(
	content: Dictionary
) -> void:
	var url: String = (
		content.get("content_base_url", "https://peer.decentraland.org/content")
		+ "entities/active"
	)
	var wearables: PackedStringArray = content.get("new_wearables", [])
	var json_payload: String = JSON.stringify({"pointers": wearables})
	var headers = ["Content-Type: application/json"]

	var promise: Promise = http_requester.request_json(
		url, HTTPClient.METHOD_POST, json_payload, headers
	)
	
	var content_result = await promise.awaiter()
	if content_result is PromiseError:
		printerr("Failing on loading wearable ", url, " reason: ", content_result.get_error())
		return
	
	# TODO: Check promise is OK
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
				wearable_cache_map[lower_pointer_fetched]["data"] = item
				wearable_cache_map[lower_pointer_fetched]["loaded"] = true
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
			wearable_cache_map[pointer]["loaded"] = true
			wearable_cache_map[pointer]["data"] = null

	var result_promise: Promise = content["promise"]
	result_promise.call_deferred("resolve")


func _process_loading_gltf(content: Dictionary) -> void:
	var content_mapping = content.get("content_mapping")
	var file_hash: String = content.get("file_hash")
	var file_path: String = content.get("file_path")
	var base_url: String = content_mapping.get("base_url", "")
	var base_path = file_path.get_base_dir()
	var local_gltf_path = "user://content/" + file_hash

	# TODO: this is temp
	var it = content.get("it", 0)
	if it > 100000:
		printerr("timeout ", file_path)
		return

	content["it"] = it + 1

	if file_hash.is_empty() or base_url.is_empty():
		printerr("hash or base_url is empty")
		return
	var file_hash_path = base_url + file_hash

	# If gltf doesn't exists locally, we request it
	if !FileAccess.file_exists(local_gltf_path):

		var absolute_file_path = local_gltf_path.replace("user:/", OS.get_user_data_dir())
		
		var request_promise = http_requester.request_file(
			file_hash_path, absolute_file_path
		)
		
		var content_result = await request_promise.awaiter()
		if content_result is PromiseError:
			printerr("Failing on loading gltf ", file_hash_path, " reason: ", content_result.get_error())
			return

	# We load the gltf from
	var gltf := GLTFDocument.new()
	var pre_gltf_state := GLTFState.new()
	pre_gltf_state.set_additional_data("placeholder_image", true)
	var base_gltf_err = gltf.append_from_file(
		local_gltf_path, pre_gltf_state, 0, OS.get_user_data_dir()
	)
	if base_gltf_err != OK:
		printerr("GLTF " + file_hash_path + " couldn't be loaded succesfully: ", base_gltf_err)
		return

	# Load gltf dependencies
	var dependencies: Array[String] = pre_gltf_state.get_additional_data("dependencies")
	var mappings: Dictionary = {}
	var promises_dependencies: Array[Promise] = []

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
			var absolute_file_path = local_image_path.replace(
				"user:/", OS.get_user_data_dir()
			)
			promises_dependencies.push_back(
				http_requester.request_file(base_url + image_hash, absolute_file_path)
			)
		mappings[uri] = "content/" + image_hash

	content["gltf_mappings"] = mappings
	
	Awaiter.all(promises_dependencies)

	# final processing
	var new_gltf := GLTFDocument.new()
	var new_gltf_state := GLTFState.new()

	new_gltf_state.set_additional_data("base_path", base_path)
	new_gltf_state.set_additional_data("mappings", content["gltf_mappings"])
	var err = new_gltf.append_from_file(
		local_gltf_path, new_gltf_state, 0, OS.get_user_data_dir()
	)

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


func _process_loading_texture(
	content: Dictionary
) -> void:
	var content_mapping = content.get("content_mapping")
	var file_hash: String = content.get("file_hash")
	var base_url: String = content_mapping.get("base_url", "")
	var local_texture_path = "user://content/" + file_hash
	if file_hash.is_empty() or base_url.is_empty():
		printerr("hash or base_url is empty")
		return
		
	var file_hash_path = base_url + file_hash
	
	if !FileAccess.file_exists(local_texture_path):
		var absolute_file_path = local_texture_path.replace(
			"user:/", OS.get_user_data_dir()
		)

		var promise: Promise = http_requester.request_file(
			file_hash_path, absolute_file_path
		)
		
		var content_result = await promise.awaiter()
		if content_result is PromiseError:
			printerr("Failing on loading gltf ", file_hash_path, " reason: ", content_result.get_error())
			return

	var file = FileAccess.open(local_texture_path, FileAccess.READ)
	if file == null:
		printerr("texture download fails")
		return

	var buf = file.get_buffer(file.get_length())
	var image := Image.new()
	var err = image.load_png_from_buffer(buf)
	if err != OK:
		printerr(
			"Texture " + base_url + file_hash + " couldn't be loaded succesfully: ", err
		)
		return

	var content_cache = content_cache_map[file_hash]
	var resource = ImageTexture.create_from_image(image)
	content_cache["image"] = image
	content_cache["resource"] = resource
	content_cache["loaded"] = true

	var promise: Promise = content_cache["promise"]
	promise.call_deferred("resolve_with_data", resource)


func _process_loading_audio(
	content: Dictionary
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

		var promise: Promise = http_requester.request_file(
			file_hash_path, absolute_file_path
		)
		var content_result = await promise.awaiter()
		if content_result is PromiseError:
			printerr("Failing on loading wearable ", file_hash_path, " reason: ", content_result.get_error())
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
		printerr(
			"Audio " + base_url + file_hash + " unrecognized format (infered by file path)"
		)
		return

	var content_cache = content_cache_map[file_hash]
	content_cache["resource"] = audio_stream
	content_cache["loaded"] = true

	var promise: Promise = content_cache["promise"]
	promise.call_deferred("resolve_with_data", audio_stream)
	return


func _process_loading_video(
	content: Dictionary
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
		var promise: Promise = http_requester.request_file(
			base_url + file_hash, absolute_file_path
		)
		var content_result = await promise.awaiter()
		if content_result is PromiseError:
			printerr("Failing on loading wearable ", file_hash_path, " reason: ", content_result.get_error())
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


func _hide_colliders(gltf_node):
	for maybe_collider in gltf_node.get_children():
		if maybe_collider is Node3D and maybe_collider.name.find("_collider") != -1:
			maybe_collider.visible = false

		if maybe_collider is Node:
			_hide_colliders(maybe_collider)


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
	return false


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
