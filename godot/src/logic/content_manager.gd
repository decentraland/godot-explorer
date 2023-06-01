extends Node

class_name ContentManager

signal content_loading_finished(hash: String)

enum ContentType {
	CT_GLTF_GLB = 1
} 

var pending_content: Array[Dictionary] = []
var content_cache_map: Dictionary = {}

var content_thread_pool: Thread = null

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
	
func get_resource(file_path: String, content_type: ContentType, content_mapping: ContentMapping):
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
			
func download_file(url: String, file: String, redirection: int = 0) -> bool:
	var http_client = HTTPClient.new()
	var parsed_url = parse_url(url)
	print("downloading file ", file)
	if not parsed_url is Dictionary:
		printerr("Error parsing url ContentManager::download_file ", parsed_url)
		return false
	
	var port = parsed_url.get("r_port")
	if port == 0:
		port = -1
		
	var tls = null
	if parsed_url.get("r_scheme") == "https":
		tls = TLSOptions.client()
		
	var err = http_client.connect_to_host(parsed_url.get("r_host"), port, tls)
	if err != OK:
		printerr("Error connecting url ContentManager::download_file ", url, " error ", err)
		return false
		
	while http_client.get_status() == HTTPClient.STATUS_CONNECTING or http_client.get_status() == HTTPClient.STATUS_RESOLVING:
		http_client.poll()
#		print("Connecting...")
		OS.delay_msec(1)

	assert(http_client.get_status() == HTTPClient.STATUS_CONNECTED) # Check if the connection was made successfully.
	print("connected to host requesting ", parsed_url["r_path"])

	var result = http_client.request(HTTPClient.METHOD_GET, parsed_url["r_path"], [])
	print("some")
	if result != OK:
		printerr("Error connecting url ContentManager::download_file ", url, " error ", err)
		return false
		
	while http_client.get_status() == HTTPClient.STATUS_REQUESTING:
		http_client.poll()
#		print("Requesting...")
		OS.delay_msec(1)

	assert(http_client.get_status() == HTTPClient.STATUS_BODY or http_client.get_status() == HTTPClient.STATUS_CONNECTED) # Make sure request finished well.
#	print("response? ", http_client.has_response()) # Site might not have a response.

	if http_client.has_response():
		var headers = http_client.get_response_headers_as_dictionary()
		print("code: ", http_client.get_response_code())
		var code = http_client.get_response_code()
		if code == 301 or code == 302:
			
			if redirection > 8:
				printerr("too many redirect ")
				return false
				
				
			var new_location = headers.get("Location")
			if new_location == null:
				printerr("no location to redirect ", headers)
			
				return false
				
			if new_location != url:
				print("redirection to ", new_location)
				http_client.close()
				return download_file(new_location, file, redirection + 1)
			else:
				printerr("wrong redirection ", new_location)
				return false
				
#		print("**headers:\\n", headers) # Show headers.

		if http_client.is_response_chunked():
			print("Response is Chunked!")
		else:
			var bl = http_client.get_response_body_length()
			print("Response Length: ", bl)
			
		if http_client.get_status() == HTTPClient.STATUS_BODY:
			var file_write = FileAccess.open(file,FileAccess.WRITE)
			if file_write == null:
				return false
				
			while http_client.get_status() == HTTPClient.STATUS_BODY:
				http_client.poll()
				var chunk: PackedByteArray = http_client.read_response_body_chunk()
				if chunk.size() == 0:
					OS.delay_msec(1)
				else:
					file_write.store_buffer(chunk)
					
			file_write.close()
		else: 
			printerr(http_client.get_status())
			return false
#		print("bytes got: ", rb.size())
#			var text = rb.get_string_from_ascii()
#			print("Text: ", text)
		
	print("ok download file ", file)
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

#func load_gltf(file_path: String, file_hash: String,  content_mapping: ContentMapping):
#	var realm: Realm = get_tree().root.get_node("realm")
#	var base_url = content_mapping.get_base_url()
#
#	if file_hash.is_empty() or base_url.is_empty():
#		return null
#
#	await realm.requester.do_request_file(base_url + file_hash, file_hash, 0)
#	var local_gltf_path = "user://content/" + file_hash
#	if not FileAccess.file_exists(local_gltf_path):
#		return null
#
#	var base_path = file_path.get_base_dir()
#	var gltf := GLTFDocument.new()
#	var gltf_state := GLTFState.new()
#	gltf_state.set_additional_data("base_path", base_path)
#	gltf_state.set_additional_data("content_mapping", content_mapping)
#	gltf_state.set_additional_data("realm", realm)
#	var err = gltf.append_from_file(local_gltf_path, gltf_state, 0, "user://")
#	if err != OK:
#		printerr("GLTF " + file_path + " couldn't be loaded succesfully: ", err)
#
#	var node = gltf.generate_scene(gltf_state)
#	if node != null:
#		hide_colliders(node)
#0
#	return node

func parse_url(base: String):
	var ret = {}
	ret["r_scheme"] = ""
	ret["r_host"] = ""
	ret["r_port"] = 0
	ret["r_path"] = ""
	var pos := base.find("://")
	# Scheme
	if pos != -1:
		ret["r_scheme"] = base.substr(0, pos + 3).to_lower()
		base = base.substr(pos + 3, base.length() - pos - 3)
	pos = base.find("/")
	# Path
	if pos != -1:
		ret["r_path"] = base.substr(pos, base.length() - pos)
		base = base.substr(0, pos)
	# Host
	pos = base.find("@")
	if pos != -1:
		# Strip credentials
		base = base.substr(pos + 1, base.length() - pos - 1)
	if base.begins_with("["):
		# Literal IPv6
		pos = base.rfind("]")
		if pos == -1:
			return ERR_INVALID_PARAMETER
		ret["r_host"] = base.substr(1, pos - 1)
		base = base.substr(pos + 1, base.length() - pos - 1)
	else:
		# Anything else
		if base.get_slice_count(":") > 2:
			return ERR_INVALID_PARAMETER
		pos = base.rfind(":")
		if pos == -1:
			ret["r_host"] = base
			base = ""
		else:
			ret["r_host"] = base.substr(0, pos)
			base = base.substr(pos, base.length() - pos)
	if ret["r_host"].is_empty():
		return ERR_INVALID_PARAMETER
	ret["r_host"] = ret["r_host"].to_lower()
	# Port
	if base.begins_with(":"):
		base = base.substr(1, base.length() - 1)
		if not base.is_valid_int():
			return ERR_INVALID_PARAMETER
		ret["r_port"] = base.to_int()
		if ret["r_port"] < 1 or ret["r_port"] > 65535:
			return ERR_INVALID_PARAMETER
	return ret
