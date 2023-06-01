extends GLTFDocumentExtension


func _import_preflight(state: GLTFState, _extensions: PackedStringArray) -> Error:
	var base_path = state.get_additional_data("base_path")

	if base_path != null and not base_path.is_empty():
		var mappings = state.get_additional_data("mappings")
		for image in state.json.get("images", []):
			var uri = image.get("uri", "")
			if not uri.is_empty():
				image["uri"]= mappings.get(uri, "assets/decentraland_logo.png")


	return OK 

#func _import_preflight(state: GLTFState, _extensions: PackedStringArray) -> Error:
#	var base_path = state.get_additional_data("base_path")
#
#	if base_path == null or base_path.is_empty():
#		return OK
#
#	var content_mapping: ContentMapping = state.get_additional_data("content_mapping")
#	var base_url: String = content_mapping.get_base_url()
#	var realm: Realm = state.get_additional_data("realm")
#
#	for image in state.json.get("images", []):
#		var uri = image.get("uri", "")
#		if not uri.is_empty():
#			var image_path = base_path + "/" + uri
#			var image_hash = content_mapping.get_content_hash(image_path.to_lower())
#			if image_hash.is_empty() or base_url.is_empty():
#				printerr(uri + " not found (resolved: " + image_path + ") => ", content_mapping.get_mappings())
#				continue
#
#			image["uri"]= "content/" + image_hash.md5_text()
#
#			await realm.requester.do_request_file(base_url + image_hash, image_hash.md5_text(), 0)
#			if not FileAccess.file_exists("user://content/" + image_hash.md5_text()):
#				continue
#
#	return OK 
#
#
#
