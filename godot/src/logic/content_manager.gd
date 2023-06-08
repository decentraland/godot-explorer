extends Node
class_name ContentManager

signal content_loading_finished(hash: String)

enum ContentType {
	CT_GLTF_GLB = 1
} 

var loading_content: Array[Dictionary] = []
var pending_content: Array[Dictionary] = []
var content_cache_map: Dictionary = {}
var content_thread_pool: Thread = null
var http_requester = RustHttpRequester.new()

func _ready():
	var custom_importer = load("res://src/logic/custom_gltf_importer.gd").new()
	GLTFDocument.register_gltf_document_extension(custom_importer)
	
	content_thread_pool = Thread.new()
	content_thread_pool.start(self.content_thread_pool_func)

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
		"content_mapping": content_mapping,
		"stage": 0
	})
	
	return true



func content_thread_pool_func():
	var to_delete = []
	var content_type: ContentType
	var finished_downloads: Array[RequestResponse] = []
	DirAccess.copy_absolute("res://decentraland_logo.png", "user://decentraland_logo.png")

	while true:
		OS.delay_msec(1)
		while pending_content.size() > 0:
			loading_content.push_back(pending_content.pop_front())
		finished_downloads = get_finished_downloads()
		if finished_downloads.size() > 0:
			# print("finished downloads ", finished_downloads.size())
			pass
		for content in loading_content:
			content_type = content.get("content_type")
			match content_type:
				ContentType.CT_GLTF_GLB:
					if not process_loading_gltf(content, finished_downloads):
						# print("deleting ", content["file_path"])
						to_delete.push_back(content)
				_: 
					printerr("Fetching invalid content type ", content_type)
		
		for item in to_delete:
			loading_content.erase(item)
			
func get_finished_downloads() -> Array[RequestResponse]:
	var ret:Array[RequestResponse] = []
	var finished_download:RequestResponse = http_requester.poll()
	while finished_download != null:
		ret.push_back(finished_download)
		finished_download = http_requester.poll()
	return ret
	
func process_loading_gltf(content: Dictionary, finished_downloads: Array[RequestResponse]) -> bool:
	var content_mapping: ContentMapping = content.get("content_mapping")
	var file_hash: String = content.get("file_hash")
	var file_path: String = content.get("file_path")
	var base_url = content_mapping.get_base_url()
	var base_path = file_path.get_base_dir()
	var local_gltf_path = "user://content/" + file_hash
	
	var stage = content.get("stage", 0)

	# TODO: this is temp
	var it = content.get("it", 0)
	if it > 100000:
		printerr("timeout ", file_path, " stage ", stage)
		return false
	
	content["it"] = it + 1
	
	
	match stage:
	# Stage 0 => request gltf/glb file
		0:
			if FileAccess.file_exists(local_gltf_path):
				content["stage"] = 2
			else:
				if file_hash.is_empty() or base_url.is_empty():
					printerr("hash or base_url is empty")
					return false
					
				var absolute_file_path = local_gltf_path.replace("user:/", OS.get_user_data_dir())
				content["stage"] = 1
				content["request_id"] = http_requester.request_file(0, base_url + file_hash, absolute_file_path)
				
	# Stage 2 => wait for the file
		1:
			for item in finished_downloads:
				if item.id() == content["request_id"]:
					if item.is_error():
						printerr("gltf download is_error() == true!")
						return false
					else:
						content["stage"] = 2
						
	# Stage 3 => process gltf/glb (and request dependencies)
		2: 
			var gltf := GLTFDocument.new()
			var pre_gltf_state := GLTFState.new()
			pre_gltf_state.set_additional_data("placeholder_image", true)
			var err = gltf.append_from_file(local_gltf_path, pre_gltf_state, 0, OS.get_user_data_dir())
			if err != OK:
				printerr("GLTF " + file_path + " couldn't be loaded succesfully: ", err)
				return false
				
			var dependencies: Array[String] = pre_gltf_state.get_additional_data("dependencies")
			var mappings: Dictionary = {}
			
			content["request_dependencies"] = []
			for uri in dependencies:
				var image_path = base_path + "/" + uri
				var image_hash = content_mapping.get_content_hash(image_path.to_lower())
				if image_hash.is_empty() or base_url.is_empty():
					printerr(uri + " not found (resolved: " + image_path + ") => ", content_mapping.get_mappings())
					continue

				var local_image_path = "user://content/" + image_hash
				if not FileAccess.file_exists(local_image_path):
					var absolute_file_path = local_image_path.replace("user:/", OS.get_user_data_dir())
					content["request_dependencies"].push_back(http_requester.request_file(0, base_url + image_hash, absolute_file_path))
				mappings[uri] = "content/" + image_hash
					
			content["gltf_mappings"] = mappings
			content["stage"] = 3

	# Stage 3 => wait for dependencies
		3:
			var dep: Array = content["request_dependencies"]
			for item in finished_downloads:
				if dep.has(item.id()):
					dep.erase(item.id())
					if item.is_error():
						printerr("dependencie download is_error() == true!")
			
			if dep.size() == 0:
				content["stage"] = 4
			
	# Stage 4 => final processing
		4:
			content["stage"] = 5
			var new_gltf := GLTFDocument.new()
			var new_gltf_state := GLTFState.new()

			new_gltf_state.set_additional_data("base_path", base_path)
			new_gltf_state.set_additional_data("mappings", content["gltf_mappings"])
			var err = new_gltf.append_from_file(local_gltf_path, new_gltf_state, 0, OS.get_user_data_dir())

			var node = new_gltf.generate_scene(new_gltf_state)
			if node != null:
				node.rotate_y(PI)
				hide_colliders(node)
				if err != OK:
					push_warning("resource with errors ", file_path, " : ", err)
			else:
				printerr("resource resolved as null ", file_path, " err?", err)
						
			content_cache_map[file_hash]["resource"] = node
			content_cache_map[file_hash]["loaded"] = true
			self.emit_signal.call_deferred("content_loading_finished", file_hash)
			return false
		_:
			printerr("unknown stage ", file_path)
			return false
	
	return true

func hide_colliders(gltf_node):
	for maybe_collider in gltf_node.get_children():
		if maybe_collider is Node3D and maybe_collider.name.ends_with("_collider"):
			maybe_collider.visible = false
		
		if maybe_collider is Node:
			hide_colliders(maybe_collider)
#
#func load_gltf(file_path: String, file_hash: String,  content_mapping: ContentMapping):
#	var base_url = content_mapping.get_base_url()
#	if file_hash.is_empty() or base_url.is_empty():
#		return null
#
#	var local_gltf_path = "user://content/" + file_hash
#	if not FileAccess.file_exists(local_gltf_path):
#		download_file(base_url + file_hash, local_gltf_path)
#		if not FileAccess.file_exists(local_gltf_path):
#			return null
#
#	var base_path = file_path.get_base_dir()
#	var gltf := GLTFDocument.new()
#	var pre_gltf_state := GLTFState.new()
#	var err = gltf.append_from_file(local_gltf_path, pre_gltf_state, 0, "res://")
#	if err != OK:
#		printerr("GLTF " + file_path + " couldn't be loaded succesfully: ", err)
#		return null
#
#	var images = pre_gltf_state.json.get("images", [])
#	var mappings: Dictionary = {}
#
#	for image in images:
#		var uri = image.get("uri", "")
#		if not uri.is_empty():
#			var image_path = base_path + "/" + uri
#			var image_hash = content_mapping.get_content_hash(image_path.to_lower())
#			if image_hash.is_empty() or base_url.is_empty():
#				printerr(uri + " not found (resolved: " + image_path + ") => ", content_mapping.get_mappings())
#				continue
#
#			var local_image_path = "user://content/" + image_hash
#			download_file(base_url + image_hash, local_image_path)
#			if not FileAccess.file_exists(local_image_path):
#				continue
#
#			mappings[uri] = "content/" + image_hash
#
#	var new_gltf := GLTFDocument.new()
#	var new_gltf_state := GLTFState.new()
#
#	new_gltf_state.set_additional_data("base_path", base_path)
#	new_gltf_state.set_additional_data("mappings", mappings)
#	new_gltf.append_from_file(local_gltf_path, new_gltf_state, 0, "user://")
#
#	var node = new_gltf.generate_scene(new_gltf_state)
#	if node != null:
#		hide_colliders(node)
#		for child in node.get_children():
#			if node is Node3D:
#				node.rotate_y(PI)
#
#	return node
