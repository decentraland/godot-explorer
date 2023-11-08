extends RefCounted
class_name AvatarRendererHelper

class ColorEntry:
	static func from_dict(value):
		if value == null or not value is Dictionary:
			printerr("color is not a dictionary", value)
			return null
		var color_dict = value.get("color", {})
		var r = color_dict.get("r")
		var g = color_dict.get("g")
		var b = color_dict.get("b")
		var a = color_dict.get("a")
		if [r,g,b].all(func(v): return v != null):
			return Color(r,g,b,a)
		else:
			printerr("some color field is not there", value)
		return null
		
class AvatarJson:
	var body_shape
	var wearables
	var eyes
	var hair
	var skin
	
	func is_valid() -> bool:
		return [body_shape, wearables, eyes, hair, skin].all(func(v): return v != null)
		
	static func from_json(value):
		if value == null or not value is Dictionary:
			printerr("avatar is not a dictionary", value)
			return 
			
		var ret:= AvatarJson.new()
		if value.get("bodyShape") is String:
			ret.body_shape = value.get("bodyShape")
		if value.get("wearables") is Array:
			ret.wearables = value.get("wearables")
		ret.eyes = ColorEntry.from_dict(value.get("eyes"))
		ret.hair = ColorEntry.from_dict(value.get("hair"))
		ret.skin = ColorEntry.from_dict(value.get("skin"))
		
		if ret.is_valid():
			return ret
		return null

class AvatarEntry:
	var dest_path
	var avatar: AvatarJson

	static func from_json(value):
		if value == null or not value is Dictionary:
			printerr("avatar is not a dictionary", value)
			return 
		
		var ret := AvatarEntry.new()
		ret.dest_path = value.get("destPath")
		ret.avatar = AvatarJson.from_json(value.get("avatar"))
		
		if ret.dest_path is String and ret.avatar != null:
			return ret
		
		return null
			
class AvatarFile:
	var base_url: String
	var payload: Array[AvatarEntry]
	
	static func from_file_path(file_path: String):
		var file = FileAccess.open(file_path,FileAccess.READ)
		if file == null:
			return null
		
		var json_value = JSON.parse_string(file.get_as_text())
		if json_value == null or not json_value is Dictionary:
			printerr("the file has to be a valid json dictionary")
			return null
			
		var base_url = json_value.get("baseUrl")
		var payload = json_value.get("payload")
		if not ([base_url, payload].all(func(v): return v != null)):
			printerr("baseUrl and payload property has to be included in the the file dictionary")
			return null
			
		if not payload is Array:
			printerr("payload has to be an array")
			return null
			
		var ret := AvatarFile.new()
			
		ret.base_url = base_url
		ret.payload = []
		for maybe_entry in payload:
			var entry = AvatarEntry.from_json(maybe_entry)
			if entry == null:
				printerr("payload entry dismissed")
				continue
			
			ret.payload.push_back(entry)
			
		return ret
