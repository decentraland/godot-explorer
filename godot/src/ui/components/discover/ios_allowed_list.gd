class_name IosAllowedList
extends RefCounted

## Fetches and caches the iOS allowed scenes list from the BFF endpoint.
## Used to filter Discover carousel results on iOS.

const BFF_URL = "https://mobile-bff.decentraland.org/places?tag=allowed_ios"

static var _allowed_parcels: Dictionary = {}
static var _allowed_worlds: Dictionary = {}
static var _loaded: bool = false
static var _loading: bool = false


static func async_load() -> void:
	if _loaded or _loading:
		return

	_loading = true
	print("[IosAllowedList] Fetching allowed list from: ", BFF_URL)

	var response = await Global.async_signed_fetch(BFF_URL, HTTPClient.METHOD_GET, "")

	if response is PromiseError:
		printerr("[IosAllowedList] Failed to fetch allowed list: ", response.get_error())
		_loading = false
		return

	var json: Dictionary = response.get_string_response_as_json()
	if not json.get("ok", false) or not json.has("data"):
		printerr("[IosAllowedList] Invalid response format")
		_loading = false
		return

	for entry in json.data:
		var entry_type: String = entry.get("type", "")
		var group: Dictionary = entry.get("group", {})

		if entry_type == "world":
			var world_name: String = group.get("name", "")
			if not world_name.is_empty():
				_allowed_worlds[world_name.to_lower()] = true
		else:
			var parcels: Array = group.get("parcels", [])
			if parcels.size() > 0:
				var parcel = parcels[0]
				var x = parcel.get("x", null)
				var y = parcel.get("y", null)
				if x != null and y != null:
					_allowed_parcels["%d,%d" % [x, y]] = true

	_loaded = true
	_loading = false
	print(
		(
			"[IosAllowedList] Loaded: %d parcels, %d worlds"
			% [_allowed_parcels.size(), _allowed_worlds.size()]
		)
	)


static func async_ensure_loaded() -> void:
	if _loaded:
		return
	if not Global.is_ios_or_emulating():
		return
	# Wait for the in-flight request to finish
	while _loading:
		await Global.get_tree().process_frame
	# If still not loaded (e.g. async_load wasn't called yet), load now
	if not _loaded:
		await async_load()


static func get_positions_query_params() -> String:
	if not Global.is_ios_or_emulating():
		return ""
	if not _loaded:
		return ""
	var keys = _allowed_parcels.keys()
	if keys.size() > 200:
		keys = keys.slice(0, 200)
	var result := ""
	for key in keys:
		result += "&positions=" + key
	return result


static func get_names_query_params() -> String:
	if not Global.is_ios_or_emulating():
		return ""
	if not _loaded:
		return ""
	var keys = _allowed_worlds.keys()
	if keys.size() > 200:
		keys = keys.slice(0, 200)
	var result := ""
	for world_name in keys:
		result += "&names=" + world_name.uri_encode()
	return result


static func is_place_allowed(place_data: Dictionary) -> bool:
	if not Global.is_ios_or_emulating():
		return true

	if not _loaded:
		print(
			"[IosAllowedList] Not loaded yet, allowing place: ",
			place_data.get("title", place_data.get("name", "unknown"))
		)
		return true

	var is_world: bool = place_data.get("world", false)
	if is_world:
		var world_name: String = place_data.get("world_name", "")
		var allowed = _allowed_worlds.has(world_name.to_lower())
		print("[IosAllowedList] World '%s' allowed=%s" % [world_name, str(allowed)])
		return allowed

	var base_position: String = place_data.get("base_position", "")
	if base_position.is_empty():
		# Fallback: check coordinates array (used by events)
		var coords = place_data.get("coordinates", [])
		if coords.size() == 2:
			base_position = "%d,%d" % [int(coords[0]), int(coords[1])]

	if base_position.is_empty():
		print(
			"[IosAllowedList] No position found for place: ",
			place_data.get("title", place_data.get("name", "unknown"))
		)
		return false

	var allowed = _allowed_parcels.has(base_position)
	print(
		(
			"[IosAllowedList] Place '%s' at %s allowed=%s"
			% [
				place_data.get("title", place_data.get("name", "unknown")),
				base_position,
				str(allowed)
			]
		)
	)
	return allowed
