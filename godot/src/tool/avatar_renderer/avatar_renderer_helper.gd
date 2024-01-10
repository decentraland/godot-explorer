class_name AvatarRendererHelper
extends RefCounted


class AvatarEntry:
	var entity := ""
	var dest_path := ""
	var width := 2048
	var height := 2048

	var face_dest_path := ""
	var face_width := 256
	var face_height := 256
	var face_zoom := 25
	var avatar: Dictionary

	static func from_json(value):
		if value == null or not value is Dictionary:
			printerr("avatar is not a dictionary", value)
			return

		var ret := AvatarEntry.new()
		ret.entity = value.get("entity", "")
		ret.dest_path = value.get("destPath", "")
		ret.width = value.get("width", 2048)
		ret.height = value.get("height", 2048)
		ret.face_dest_path = value.get("faceDestPath", "")  # optional
		ret.face_width = value.get("faceWidth", 256)
		ret.face_height = value.get("faceHeight", 256)
		ret.face_zoom = value.get("faceZoom", 25)
		ret.avatar = value.get("avatar")

		if ret.dest_path is String and ret.avatar != null:
			return ret

		return null


class AvatarFile:
	var base_url: String
	var payload: Array[AvatarEntry]

	static func from_file_path(file_path: String):
		var file = FileAccess.open(file_path, FileAccess.READ)
		if file == null:
			return null

		var json_value = JSON.parse_string(file.get_as_text())
		if json_value == null or not json_value is Dictionary:
			printerr("the file has to be a valid json dictionary")
			return null

		var tmp_base_url = json_value.get("baseUrl")
		var tmp_payload = json_value.get("payload")
		if not ([tmp_base_url, tmp_payload].all(func(v): return v != null)):
			printerr("baseUrl and payload property has to be included in the the file dictionary")
			return null

		if not tmp_payload is Array:
			printerr("payload has to be an array")
			return null

		var ret := AvatarFile.new()

		ret.base_url = tmp_base_url
		ret.payload = []
		for maybe_entry in tmp_payload:
			var entry = AvatarEntry.from_json(maybe_entry)
			if entry == null:
				printerr("payload entry dismissed")
				continue

			ret.payload.push_back(entry)

		return ret
