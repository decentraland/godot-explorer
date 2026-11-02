extends GLTFDocumentExtension

const KTX2_PLACEHOLDER_PNG = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg=="


func _import_preflight(state: GLTFState, _extensions: PackedStringArray) -> Error:
	var placeholder_image = state.get_additional_data("placeholder_image")
	if placeholder_image:
		var dependencies: Array[String] = []
		for image in state.json.get("images", []):
			var uri = image.get("uri", "")
			if not uri.is_empty() and not uri.begins_with("data:"):
				dependencies.push_back(String(uri))
				image["uri"] = "decentraland_logo.png"

		for buf in state.json.get("buffers", []):
			var uri = buf.get("uri", "")
			if not uri.is_empty() and not uri.begins_with("data:"):
				dependencies.push_back(String(uri))
				buf["uri"] = "empty_buffer.bin"

		state.set_additional_data("dependencies", dependencies)
	else:
		var base_path = state.get_additional_data("base_path")
		if base_path != null:
			var mappings = state.get_additional_data("mappings")
			for image in state.json.get("images", []):
				var uri = image.get("uri", "")
				if not uri.is_empty() and not uri.begins_with("data:"):
					var mapped = mappings.get(uri, uri)
					if (
						image.get("mimeType", "") == "image/ktx2"
						or uri.get_extension().to_lower() == "ktx2"
					):
						# The importer cannot decode KTX2 from an extension-less cache file; it gets a
						# 1x1 PNG so the material has a texture, and Rust swaps in the real image.
						image["extras"] = {"dcl_src_uri": mapped}
						image["mimeType"] = "image/png"
						mapped = KTX2_PLACEHOLDER_PNG
					image["uri"] = mapped
			for buf in state.json.get("buffers", []):
				var uri = buf.get("uri", "")
				if not uri.is_empty() and not uri.begins_with("data:"):
					buf["uri"] = mappings.get(uri, uri)
	return OK
