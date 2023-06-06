extends GLTFDocumentExtension

func _import_preflight(state: GLTFState, _extensions: PackedStringArray) -> Error:
	var placeholder_image = state.get_additional_data("placeholder_image")
	if placeholder_image:
		var dependencies: Array[String] = []
		for image in state.json.get("images", []):
			var uri = image.get("uri", "")
			if not uri.is_empty():
				dependencies.push_back(String(uri))
				image["uri"] = "decentraland_logo.png"
		state.set_additional_data("dependencies", dependencies)
	else:
		var base_path = state.get_additional_data("base_path")
		if base_path != null and not base_path.is_empty():
			var mappings = state.get_additional_data("mappings")
			for image in state.json.get("images", []):
				var uri = image.get("uri", "")
				if not uri.is_empty():
					image["uri"]= mappings.get(uri, "decentraland_logo.png")
	return OK 
