extends Node

class_name ContentManager

signal internal_call_download_file(url: String, file: String)

signal content_loading_finished(hash: String)

enum ContentType {
	CT_GLTF_GLB = 1
} 

var pending_content: Array[Dictionary] = []
var content_cache_map: Dictionary = {}

var content_thread_pool: Thread = null

var http_many_requester: HTTPManyRequester

var downloading_file: Dictionary = {}

func _ready():
	var custom_importer = load("res://src/logic/custom_gltf_importer.gd").new()
	GLTFDocument.register_gltf_document_extension(custom_importer)
	

	http_many_requester = HTTPManyRequester.new()
	http_many_requester.name = "http_many_requester_parcel"
	http_many_requester.request_completed.connect(self._on_requested_completed)
	add_child(http_many_requester)
	
	content_thread_pool = Thread.new()
	content_thread_pool.start(self.content_thread_pool_func)
	
	self.internal_call_download_file.connect(self._on_internal_call_download_file)
	
func _on_requested_completed(reference_id: int, request_id: String, result: int, response_code: int, headers: PackedStringArray, body: PackedByteArray):
	for file in downloading_file.values():
		if request_id == file["request_id"]:
			if result == OK and response_code >= 200 and response_code <= 299:
				file["success"] = true
			file["completed"] = true
			return
	
func _on_internal_call_download_file(url: String, file: String):
	var request_id = http_many_requester.request(0, url, HTTPClient.METHOD_GET, "", [], 0, file)
	downloading_file[file] = {
		"request_id": request_id,
		"completed": false
	}
	
func _process(dt: float):
	pass
	

func get_resource_from_hash(file_hash: String):
	var content_cached = content_cache_map.get(file_hash)
	if content_cached != null and content_cached.get("loaded"):
		return content_cached.get("resource")
	return null
	
func get_resource(file_path: String, _content_type: ContentType, content_mapping: ContentMapping):
	var file_hash = content_mapping.get_content_hash(file_path)
	var content_cached = content_cache_map.get(file_hash)
	if content_cached != null and content_cached.get("loaded"):
		return content_cached.get("resource")
		
	return null
		
func fetch_resource(file_path: String, content_type: ContentType, content_mapping: ContentMapping):
	var file_hash = content_mapping.get_content_hash(file_path)
	var content_cached = content_cache_map.get(file_hash)
	if content_cached != null:
		return not content_cached.get("loaded")
	
	content_cache_map[file_hash] = {
		"loaded": false,
	}
	
	pending_content.push_back({
		"file_path": file_path,
		"file_hash": file_hash,
		"content_type": content_type,
		"content_mapping": content_mapping
	})
	
	return true
	
func content_thread_pool_func():
	DirAccess.copy_absolute("res://assets/decentraland_logo.png", "user://decentraland_logo.png")
	
	var is_pending_content: bool = false
	while true:
		is_pending_content = pending_content.size() > 0
		while is_pending_content:
			var content: Dictionary = pending_content.pop_front()
			
			var content_type: ContentType = content.get("content_type")
			var content_mapping: ContentMapping = content.get("content_mapping")
			var file_hash: String = content.get("file_hash")
			var file_path: String = content.get("file_path")
			
			print("Fetching content ", file_path , " with hash ", file_hash)
			
			match content_type:
				ContentType.CT_GLTF_GLB:
					var resource = load_gltf(file_path, file_hash, content_mapping)
					content_cache_map[file_hash]["resource"] = resource
					content_cache_map[file_hash]["loaded"] = true
					self.emit_signal("content_loading_finished", file_hash)
				_: 
					printerr("Fetching invalid content type ", content_type)
					
			is_pending_content = pending_content.size() > 0
			
		OS.delay_msec(1)
		
func hide_colliders(gltf_node):
	for maybe_collider in gltf_node.get_children():
		if maybe_collider is Node3D and maybe_collider.name.ends_with("_collider"):
			maybe_collider.visible = false
		
		if maybe_collider is Node:
			hide_colliders(maybe_collider)
			
func download_file(url: String, file: String) -> bool:
	emit_signal.call_deferred("internal_call_download_file", url, file)
	OS.delay_msec(1)
	while downloading_file.get(file) == null:
		OS.delay_msec(1)
		
	while downloading_file.get(file).get("completed", false) == false:
		OS.delay_msec(1)
		
	return true
	
func load_gltf(file_path: String, file_hash: String,  content_mapping: ContentMapping):
	var base_url = content_mapping.get_base_url()

	if file_hash.is_empty() or base_url.is_empty():
		return null

	var local_gltf_path = "user://content/" + file_hash
	if not FileAccess.file_exists(local_gltf_path):
		download_file(base_url + file_hash, local_gltf_path)
		if not FileAccess.file_exists(local_gltf_path):
			return null

	var base_path = file_path.get_base_dir()
	var gltf := GLTFDocument.new()
	var pre_gltf_state := GLTFState.new()
	var err = gltf.append_from_file(local_gltf_path, pre_gltf_state, 0, "res://")
	if err != OK:
		printerr("GLTF " + file_path + " couldn't be loaded succesfully: ", err)
		return null
		
	var images = pre_gltf_state.json.get("images", [])
	var mappings: Dictionary = {}

	for image in images:
		var uri = image.get("uri", "")
		if not uri.is_empty():
			var image_path = base_path + "/" + uri
			var image_hash = content_mapping.get_content_hash(image_path.to_lower())
			if image_hash.is_empty() or base_url.is_empty():
				printerr(uri + " not found (resolved: " + image_path + ") => ", content_mapping.get_mappings())
				continue

			var local_image_path = "user://content/" + image_hash
			download_file(base_url + image_hash, local_image_path)
			if not FileAccess.file_exists(local_image_path):
				continue

			mappings[uri] = "content/" + image_hash

	var new_gltf := GLTFDocument.new()
	var new_gltf_state := GLTFState.new()

	new_gltf_state.set_additional_data("base_path", base_path)
	new_gltf_state.set_additional_data("mappings", mappings)
	new_gltf.append_from_file(local_gltf_path, new_gltf_state, 0, "user://")

	var node = new_gltf.generate_scene(new_gltf_state)
	if node != null:
		hide_colliders(node)
		for child in node.get_children():
			if node is Node3D:
				node.rotate_y(PI)

	return node
