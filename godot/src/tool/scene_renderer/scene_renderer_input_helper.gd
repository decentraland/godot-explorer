class_name SceneRendererInputHelper
extends RefCounted


class CameraOption:
	var position: Vector3
	var target: Vector3
	var fov: float
	var ortho_size: float
	var projection: String

	static func from_dictionary(value: Dictionary, default: CameraOption) -> CameraOption:
		var ret = CameraOption.new()

		ret.position = Vector3(
			value.get("position", {}).get("x", default.position.x),
			value.get("position", {}).get("y", default.position.y),
			value.get("position", {}).get("z", default.position.z)
		)

		ret.target = Vector3(
			value.get("target", {}).get("x", default.target.x),
			value.get("target", {}).get("y", default.target.y),
			value.get("target", {}).get("z", default.target.z)
		)

		ret.projection = value.get("projection", default.projection)
		ret.fov = value.get("fov", default.fov)
		ret.ortho_size = value.get("orthoSize", default.ortho_size)

		return ret


class SceneRendererInputSpecs:
	var coords := Vector2i(-9999, -9999)
	var scene_distance := 0
	var width := 2048
	var height := 2048
	var dest_path := ""
	var camera := CameraOption.new()
	var index: int = -1

	static func from_dictionary(
		value: Dictionary, default: SceneRendererInputSpecs
	) -> SceneRendererInputSpecs:
		var ret = SceneRendererInputSpecs.new()

		var coord_array = value.get("coords", "").split(",")
		if coord_array.size() == 2:
			ret.coords = Vector2i(int(coord_array[0]), int(coord_array[1]))
		else:
			ret.coords = default.coords

		ret.dest_path = value.get("destPath", default.dest_path)
		ret.scene_distance = value.get("sceneDistance", default.scene_distance)
		ret.width = value.get("width", default.width)
		ret.height = value.get("height", default.height)
		ret.camera = CameraOption.from_dictionary(value.get("camera", {}), default.camera)

		return ret


class SceneInputFile:
	var realm_url: String
	var scenes: Array[SceneRendererInputSpecs]

	static func from_file_path(file_path: String):
		var file = FileAccess.open(file_path, FileAccess.READ)
		if file == null:
			return null

		var json_value = JSON.parse_string(file.get_as_text())
		if json_value == null or not json_value is Dictionary:
			printerr("the file has to be a valid json dictionary")
			return null

		var tmp_realm_url = json_value.get("realmUrl")
		var tmp_payload = json_value.get("payload")
		var default_payload = SceneRendererInputSpecs.from_dictionary(
			json_value.get("defaultPayload", {}), SceneRendererInputSpecs.new()
		)
		if not ([tmp_realm_url, tmp_payload].all(func(v): return v != null)):
			printerr("baseUrl and payload property has to be included in the the file dictionary")
			return null

		if not tmp_payload is Array:
			printerr("payload has to be an array")
			return null

		var ret := SceneInputFile.new()

		ret.realm_url = tmp_realm_url
		ret.scenes = []
		var i = 0
		for maybe_entry in tmp_payload:
			var scene: SceneRendererInputSpecs = SceneRendererInputSpecs.from_dictionary(
				maybe_entry, default_payload
			)
			if scene != null:
				scene.index = i
				ret.scenes.push_back(scene)
				i += 1

		return ret
