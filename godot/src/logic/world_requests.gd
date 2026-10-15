class_name WorldRequests
extends RefCounted

## Scene-entity lookup, teleport/join entry points and the signed fetch helper.


static func async_resolve_scene_entity_id(coord: Vector2i) -> String:
	# Try cache first
	var cached = Global.scene_fetcher.scene_entity_coordinator.get_scene_entity_id(coord)
	if not cached.is_empty():
		return cached

	# Make HTTP request to entities/active
	var content_url = Global.realm.content_base_url
	if content_url.is_empty():
		return ""

	var url = content_url.trim_suffix("/") + "/entities/active"
	var body = JSON.stringify({"pointers": [str(coord.x) + "," + str(coord.y)]})
	var headers = {"Content-Type": "application/json"}
	var promise: Promise = Global.http_requester.request_json(
		url, HTTPClient.METHOD_POST, body, headers
	)
	var result = await PromiseUtils.async_awaiter(promise)

	if result is PromiseError:
		push_warning("Failed to resolve scene entity ID: " + result.get_error())
		return ""

	var json = result.get_string_response_as_json()
	if json is Array and not json.is_empty():
		return json[0].get("id", "")
	return ""


static func async_teleport_to(parcel_position: Vector2i, new_realm: String) -> void:
	var explorer = Global.get_explorer()
	if not is_instance_valid(explorer):
		# Cold start from the lobby: there is no scene to protect and no explorer to route
		# through, so the private-world gate stays here — booting straight into a world the
		# user can't enter is the bug #1725 fixed.
		if not await Global._async_precheck_realm_access(new_realm):
			return
		Global.set_orientation_landscape()
		Global.get_config().last_realm_joined = new_realm
		Global.get_config().last_parcel_position = parcel_position
		Global.get_config().add_place_to_last_places(parcel_position, new_realm)
		Global.get_tree().change_scene_to_file("res://src/ui/explorer.tscn")
		return

	var dest := Destination.from_input(new_realm, parcel_position)
	if await Navigator.async_go(dest, "on_teleport"):
		Global.on_chat_message.emit(
			"system",
			Global.tr("CHAT_SYSTEM_TELEPORTED").format({"location": str(parcel_position)}),
			Time.get_unix_time_from_system()
		)


static func async_join_world(world_realm: String) -> void:
	var explorer = Global.get_explorer()
	if not is_instance_valid(explorer):
		# Cold start from the lobby: no explorer to route through, so the private-world gate
		# stays here rather than booting straight into a world we can't enter (#1725).
		if not await Global._async_precheck_realm_access(world_realm):
			return
		Global.get_config().add_place_to_last_places(Vector2i.ZERO, world_realm)  # "Last visited"
		Global.set_orientation_landscape()
		Global.get_config().last_realm_joined = world_realm
		Global.get_config().last_parcel_position = Vector2i.ZERO
		Global.get_tree().change_scene_to_file("res://src/ui/explorer.tscn")
		return

	Global.on_chat_message.emit(
		"system",
		Global.tr("CHAT_SYSTEM_CHANGING_WORLD").format({"world": world_realm}),
		Time.get_unix_time_from_system()
	)
	# No parcel: the world names its own spawn point, so teleport_to never records it.
	if await Navigator.async_go(Destination.from_input(world_realm), "on_world"):
		Global.get_config().add_place_to_last_places(Vector2i.ZERO, world_realm)  # "Last visited"


static func _http_method_to_string(method: int) -> String:
	match method:
		HTTPClient.METHOD_GET:
			return "GET"
		HTTPClient.METHOD_POST:
			return "POST"
		HTTPClient.METHOD_PUT:
			return "PUT"
		HTTPClient.METHOD_DELETE:
			return "DELETE"
		HTTPClient.METHOD_HEAD:
			return "HEAD"
		HTTPClient.METHOD_OPTIONS:
			return "OPTIONS"
		HTTPClient.METHOD_PATCH:
			return "PATCH"
		HTTPClient.METHOD_CONNECT:
			return "CONNECT"
		HTTPClient.METHOD_TRACE:
			return "TRACE"
		_:
			return "GET"  # Default fallback


static func async_signed_fetch(
	url: String,
	method: int,
	_body: String = "",
	_metadata_override: String = "",
	_extra_headers: Dictionary = {}
):
	# Decentraland signed-fetch (ADR-44) carries the request metadata in the
	# x-identity-metadata header. The server verifier requires it to be a JSON
	# object: a bodyless request would otherwise be signed as `null`, which the
	# credits-server crypto-middleware (>=4.0.0) now rejects with
	# "Invalid chain metadata". Sign an empty object `{}` for bodyless requests
	# (backward-compatible: older verifiers accept both), leaving the actual HTTP
	# body untouched.
	#
	# Most DCL services verify the body as the signed metadata. Some don't: the
	# intercom-proxy signs `{}` and leaves the JSON body unsigned, so callers can
	# override rather than fork this helper.
	var metadata := _metadata_override
	if metadata.is_empty():
		metadata = _body if not _body.is_empty() else "{}"
	var headers_promise = Global.player_identity.async_get_identity_headers(
		url, metadata, _http_method_to_string(method)
	)
	var headers_result = await PromiseUtils.async_awaiter(headers_promise)

	if headers_result is PromiseError:
		return headers_result

	var headers: Dictionary = headers_result
	if not _body.is_empty():
		headers["Content-Type"] = "application/json"
	for key in _extra_headers:
		headers[key] = _extra_headers[key]

	var response_promise: Promise = Global.http_requester.request_json(url, method, _body, headers)

	return await PromiseUtils.async_awaiter(response_promise)
