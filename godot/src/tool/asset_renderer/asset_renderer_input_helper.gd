class_name AssetRendererInputHelper
extends RefCounted

const VALID_KINDS = ["wearable_standalone", "wearable_on_avatar", "emote"]
## Bounds for a requested render size. Zero would produce a dead viewport whose
## only symptom is the 60s watchdog, and an absurd side would try to allocate
## gigabytes; both are caller mistakes worth correcting rather than obeying.
const MIN_SHOT_SIDE = 16
const MAX_SHOT_SIDE = 4096


## The input file is untrusted JSON: any field can hold any type. Reads are
## coerced rather than cast so a bad value is reported per item instead of
## raising at the assignment and taking the whole batch down.
class Coerce:
	static func as_float(value, fallback: float) -> float:
		if value is float or value is int:
			return float(value)
		return fallback

	static func as_int(value, fallback: int) -> int:
		if value is float or value is int:
			return int(value)
		return fallback

	static func as_bool(value, fallback: bool) -> bool:
		if value is bool:
			return value
		return fallback

	static func as_string(value, fallback: String) -> String:
		if value is String:
			return value
		return fallback

	static func as_vector3(value, fallback: Vector3) -> Vector3:
		if not value is Dictionary:
			return fallback
		return Vector3(
			Coerce.as_float(value.get("x"), fallback.x),
			Coerce.as_float(value.get("y"), fallback.y),
			Coerce.as_float(value.get("z"), fallback.z)
		)


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
		ret.auto_fit = Coerce.as_bool(value.get("autoFit"), default.auto_fit)
		ret.orbit_yaw_degrees = Coerce.as_float(
			value.get("orbitYawDegrees"), default.orbit_yaw_degrees
		)
		ret.orbit_pitch_degrees = Coerce.as_float(
			value.get("orbitPitchDegrees"), default.orbit_pitch_degrees
		)
		ret.fit_margin = Coerce.as_float(value.get("fitMargin"), default.fit_margin)
		ret.projection = Coerce.as_string(value.get("projection"), default.projection)
		ret.fov = Coerce.as_float(value.get("fov"), default.fov)
		ret.ortho_size = Coerce.as_float(value.get("orthoSize"), default.ortho_size)
		ret.position = Coerce.as_vector3(value.get("position"), default.position)
		ret.target = Coerce.as_vector3(value.get("target"), default.target)
		return ret


class Shot:
	var name := ""
	var dest_path := ""
	var width := 1024
	var height := 1024
	## Emotes only: normalized animation time (0..1) to freeze at.
	var at_time := 0.5
	var camera := ShotCamera.new()

	## Returns the parsed shot, or a String describing why it is invalid, so the
	## report names the actual problem rather than one stand-in reason.
	static func from_dictionary(value: Dictionary):
		var ret = Shot.new()
		ret.name = Coerce.as_string(value.get("name"), "")
		ret.dest_path = Coerce.as_string(value.get("destPath"), "")
		ret.width = clampi(Coerce.as_int(value.get("width"), 1024), MIN_SHOT_SIDE, MAX_SHOT_SIDE)
		ret.height = clampi(Coerce.as_int(value.get("height"), 1024), MIN_SHOT_SIDE, MAX_SHOT_SIDE)
		ret.at_time = Coerce.as_float(value.get("atTime"), 0.5)

		var camera_value = value.get("camera", {})
		if not camera_value is Dictionary:
			return "shot %s has a camera that is not an object" % ret.name
		ret.camera = ShotCamera.from_dictionary(camera_value, ShotCamera.new())

		if ret.dest_path.is_empty():
			return "shot %s is missing destPath" % ret.name
		# An explicit camera pointing at its own position leaves look_at unable
		# to orient, which would silently reuse the previous shot's framing.
		if not ret.camera.auto_fit and ret.camera.position.is_equal_approx(ret.camera.target):
			return "shot %s needs a distinct camera position and target" % ret.name
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
		ret.id = Coerce.as_string(value.get("id"), "")
		ret.kind = Coerce.as_string(value.get("kind"), "wearable_standalone")
		ret.urn = Coerce.as_string(value.get("urn"), "")
		ret.body_shape = Coerce.as_string(
			value.get("bodyShape"), "urn:decentraland:off-chain:base-avatars:BaseFemale"
		)
		var overrides_value = value.get("avatar", {})
		ret.avatar_overrides = overrides_value if overrides_value is Dictionary else {}

		# The report is keyed by id: without one its entry cannot be matched back
		# to the request, which reads to the caller as a missing result.
		if ret.id.is_empty():
			return "item needs an id"
		if ret.urn.is_empty():
			return "item needs a urn"
		if not ret.kind in VALID_KINDS:
			return "invalid kind %s" % ret.kind

		var shots_value = value.get("shots", [])
		if not shots_value is Array:
			return "shots has to be an array"
		for maybe_shot in shots_value:
			if not maybe_shot is Dictionary:
				return "a shot is not an object"
			var shot = Shot.from_dictionary(maybe_shot)
			if shot is String:
				return shot
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
		ret.base_url = Coerce.as_string(tmp_base_url, "")
		ret.output_json_path = Coerce.as_string(tmp_output_json_path, "")
		if ret.base_url.is_empty() or ret.output_json_path.is_empty():
			printerr("baseUrl and outputJsonPath have to be strings")
			return null
		ret.supersample = Coerce.as_float(json_value.get("supersample"), 1.0)

		var index := 0
		for maybe_entry in tmp_payload:
			var id_value := "payload[%d]" % index
			if not maybe_entry is Dictionary:
				printerr("asset item %s is invalid: entry is not an object" % id_value)
				ret.invalid_items.push_back({"id": id_value, "error": "entry is not an object"})
				index += 1
				continue

			var parsed = AssetItem.from_dictionary(maybe_entry)
			if parsed is AssetItem:
				ret.items.push_back(parsed)
			else:
				var declared_id := Coerce.as_string(maybe_entry.get("id"), "")
				if not declared_id.is_empty():
					id_value = declared_id
				printerr("asset item %s is invalid: %s" % [id_value, parsed])
				ret.invalid_items.push_back({"id": id_value, "error": str(parsed)})
			index += 1

		return ret
