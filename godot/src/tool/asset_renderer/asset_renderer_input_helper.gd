class_name AssetRendererInputHelper
extends RefCounted

const VALID_KINDS = ["wearable_standalone", "wearable_on_avatar", "emote"]


class ShotCamera:
	## When true the camera orbits the asset's posed AABB; explicit
	## position/target (scene-renderer semantics) are used otherwise.
	var auto_fit := true
	var orbit_yaw_degrees := 0.0
	var orbit_pitch_degrees := 10.0
	var fit_margin := 1.15
	var projection := "perspective"
	var fov := 40.0
	var ortho_size := 2.5
	var position := Vector3.ZERO
	var target := Vector3.ZERO

	static func from_dictionary(value: Dictionary, default: ShotCamera) -> ShotCamera:
		var ret = ShotCamera.new()
		ret.auto_fit = value.get("autoFit", default.auto_fit)
		ret.orbit_yaw_degrees = value.get("orbitYawDegrees", default.orbit_yaw_degrees)
		ret.orbit_pitch_degrees = value.get("orbitPitchDegrees", default.orbit_pitch_degrees)
		ret.fit_margin = value.get("fitMargin", default.fit_margin)
		ret.projection = value.get("projection", default.projection)
		ret.fov = value.get("fov", default.fov)
		ret.ortho_size = value.get("orthoSize", default.ortho_size)
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
		return ret


class Shot:
	var name := ""
	var dest_path := ""
	var width := 1024
	var height := 1024
	## Emotes only: normalized animation time (0..1) to freeze at.
	var at_time := 0.5
	var camera := ShotCamera.new()

	static func from_dictionary(value: Dictionary) -> Shot:
		var ret = Shot.new()
		ret.name = value.get("name", "")
		ret.dest_path = value.get("destPath", "")
		ret.width = value.get("width", 1024)
		ret.height = value.get("height", 1024)
		ret.at_time = value.get("atTime", 0.5)
		ret.camera = ShotCamera.from_dictionary(value.get("camera", {}), ShotCamera.new())
		if ret.dest_path.is_empty() or ret.camera == null:
			return null
		# An explicit camera pointing at its own position leaves look_at unable
		# to orient, which would silently reuse the previous shot's framing.
		if not ret.camera.auto_fit and ret.camera.position.is_equal_approx(ret.camera.target):
			printerr("shot %s: an explicit camera needs distinct position and target" % ret.name)
			return null
		return ret


class AssetItem:
	var id := ""
	var kind := ""
	var urn := ""
	var body_shape := ""
	## Optional AvatarWireFormat overrides merged into the synthesized avatar
	## (wearable_on_avatar only).
	var avatar_overrides := {}
	var shots: Array[Shot] = []

	## Returns the parsed item, or a String describing why it is invalid — an
	## unusable item is reported in the output report rather than failing the
	## whole batch.
	static func from_dictionary(value: Dictionary):
		var ret = AssetItem.new()
		ret.id = value.get("id", "")
		ret.kind = value.get("kind", "wearable_standalone")
		ret.urn = value.get("urn", "")
		ret.body_shape = value.get(
			"bodyShape", "urn:decentraland:off-chain:base-avatars:BaseFemale"
		)
		ret.avatar_overrides = value.get("avatar", {})

		if ret.urn.is_empty():
			return "item needs a urn"
		if not ret.kind in VALID_KINDS:
			return "invalid kind %s" % ret.kind

		var shots_value = value.get("shots", [])
		if not shots_value is Array:
			return "shots has to be an array"
		for maybe_shot in shots_value:
			var shot: Shot = Shot.from_dictionary(maybe_shot)
			if shot == null:
				return "a shot is missing destPath"
			ret.shots.push_back(shot)
		if ret.shots.is_empty():
			return "no shots"

		return ret


class AssetInputFile:
	var base_url := ""
	var output_json_path := ""
	## Maps to viewport scaling_3d_scale (clamped to [0.5, 2.0]); 1.0 keeps the
	## llvmpipe raster cost proportional to the requested resolution.
	var supersample := 1.0
	var items: Array[AssetItem] = []
	## Entries that could not be parsed: [{id, error}], reported as failures.
	var invalid_items: Array[Dictionary] = []

	static func from_file_path(file_path: String):
		var file = FileAccess.open(file_path, FileAccess.READ)
		if file == null:
			return null

		var json_value = JSON.parse_string(file.get_as_text())
		if json_value == null or not json_value is Dictionary:
			printerr("the file has to be a valid json dictionary")
			return null

		var tmp_base_url = json_value.get("baseUrl")
		var tmp_output_json_path = json_value.get("outputJsonPath")
		var tmp_payload = json_value.get("payload")
		if not ([tmp_base_url, tmp_output_json_path, tmp_payload].all(func(v): return v != null)):
			printerr("baseUrl, outputJsonPath and payload have to be included in the file")
			return null

		if not tmp_payload is Array:
			printerr("payload has to be an array")
			return null

		var ret := AssetInputFile.new()
		ret.base_url = tmp_base_url
		ret.output_json_path = tmp_output_json_path
		ret.supersample = json_value.get("supersample", 1.0)
		var index := 0
		for maybe_entry in tmp_payload:
			var parsed = AssetItem.from_dictionary(maybe_entry)
			if parsed is AssetItem:
				ret.items.push_back(parsed)
			else:
				var id_value = ""
				if maybe_entry is Dictionary:
					id_value = maybe_entry.get("id", "")
				if id_value.is_empty():
					id_value = "payload[%d]" % index
				printerr("asset item %s is invalid: %s" % [id_value, parsed])
				ret.invalid_items.push_back({"id": id_value, "error": str(parsed)})
			index += 1

		return ret
