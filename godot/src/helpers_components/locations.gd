extends Node

signal in_genesis_city_changed(players: Array)
signal online_locations_changed

const MAX_CONCURRENT_WORLD_REQUESTS: int = 8
const WORLD_BATCH_TIMEOUT: float = 15.0
# Don't re-hit a failing world-title lookup on every online_locations_changed emit.
const WORLD_TITLE_RETRY_COOLDOWN: float = 60.0

var known_locations: Array = []  # Array of objects {coord: [x,y], title: String}
var known_worlds: Dictionary = {}  # world_name -> place title (cache; mirrors known_locations)
var in_genesis_city: Array = []  # Array of objects {address: String, parcel: [int, int]}
# address_lower -> {parcel: [x, y]} (Genesis City) or {world_name: String} (a World).
# Single source of truth for online friend locations, mirroring the Discover friends carousel
# and unity-explorer's OnlineUsersProvider (+ world decorator): consumers read this and listen
# to `online_locations_changed` instead of each firing its own per-friend world request.
var online_locations: Dictionary = {}
var _online_locations_in_progress: bool = false
# World-title lookups are shared across all rows in the same world: dedupe concurrent requests
# (in-flight) and remember failures with a cooldown so an unresolvable world doesn't re-fire a
# request per row on every `online_locations_changed` emit.
var _world_title_in_progress: Dictionary = {}  # world_name -> true
var _world_title_failed_at: Dictionary = {}  # world_name -> Time.get_ticks_msec() of last failure


func fetch_peers():
	var http_request = HTTPRequest.new()
	add_child(http_request)
	http_request.request_completed.connect(_on_request_completed.bind(http_request))

	var error = http_request.request(DclUrls.archipelago_stats() + "/comms/peers")
	if error != OK:
		push_error("Error making request: " + str(error))


func _on_request_completed(
	result: int,
	response_code: int,
	_headers: PackedStringArray,
	body: PackedByteArray,
	http_request: HTTPRequest
):
	# Clean up HTTPRequest after using it
	if http_request:
		http_request.queue_free()

	# Verify that the response is OK
	if result != HTTPRequest.RESULT_SUCCESS:
		push_error("Error in HTTP request: " + str(result))
		in_genesis_city_changed.emit(in_genesis_city)
		return

	if response_code != 200:
		push_error("Error in response code: " + str(response_code))
		in_genesis_city_changed.emit(in_genesis_city)
		return

	# Parse the JSON
	var response = body.get_string_from_utf8()
	var json = JSON.new()
	var parse_result = json.parse(response)

	if parse_result != OK:
		push_error("Error parsing JSON: " + json.get_error_message())
		in_genesis_city_changed.emit(in_genesis_city)
		return

	var data = json.get_data()
	if not data.has("peers"):
		push_error("Response does not contain 'peers'")
		in_genesis_city_changed.emit(in_genesis_city)
		return

	# Create the online_players array
	in_genesis_city.clear()
	var peers = data["peers"]

	for peer in peers:
		if peer.has("address") and peer.has("parcel"):
			var parcel = _parse_parcel(peer["parcel"])
			if parcel == null:
				continue
			in_genesis_city.append({"address": str(peer["address"]), "parcel": parcel})

	in_genesis_city_changed.emit(in_genesis_city)


## Safely converts a raw archipelago "parcel" field ([x, y]) to an [int, int] array, or null when
## it is malformed (missing, wrong length, or a non-numeric / null entry). Guards int() against
## JSON nulls and nested values, which otherwise raise "Nonexistent 'int' constructor" at runtime.
func _parse_parcel(raw) -> Variant:
	if not (raw is Array) or raw.size() < 2:
		return null
	var x = _to_coord(raw[0])
	var y = _to_coord(raw[1])
	if x == null or y == null:
		return null
	return [x, y]


func _to_coord(value) -> Variant:
	if value is int or value is float:
		return int(value)
	if value is String and value.is_valid_float():
		return int(float(value))
	return null


## Resolves each address's current location (Genesis parcel or World) in one pass and publishes
## it to `online_locations`, emitting `online_locations_changed`. Genesis comes from a single
## archipelago /comms/peers request; Worlds are resolved per-address (8 concurrent), same as the
## Discover friends carousel. Call this with the set of ONLINE friend addresses on each refresh.
func async_update_online_locations(addresses: Array) -> void:
	if _online_locations_in_progress:
		return
	if addresses.is_empty():
		online_locations = {}
		online_locations_changed.emit()
		return
	_online_locations_in_progress = true

	var wanted: Dictionary = {}
	for address in addresses:
		wanted[str(address).to_lower()] = true

	var new_locations: Dictionary = {}

	# Stage 1: Genesis City (single bulk request).
	var peers: Array = await _async_fetch_peers_raw()
	for peer in peers:
		if not (peer.has("address") and peer.has("parcel")):
			continue
		var peer_address: String = str(peer["address"]).to_lower()
		if wanted.has(peer_address):
			var parcel = _parse_parcel(peer["parcel"])
			if parcel == null:
				continue
			new_locations[peer_address] = {"parcel": parcel}

	# Publish Genesis right away so those rows get their place/jump without waiting on the Worlds
	# stage (which can take up to WORLD_BATCH_TIMEOUT). Worlds are merged and re-published below.
	online_locations = new_locations
	online_locations_changed.emit()

	# Stage 2: Worlds for those not in Genesis (per-address, throttled).
	var not_in_genesis: Array = []
	for address in wanted.keys():
		if not new_locations.has(address):
			not_in_genesis.append(address)

	if not not_in_genesis.is_empty():
		var worlds: Dictionary = await _async_resolve_worlds(not_in_genesis)
		for address in worlds.keys():
			new_locations[address] = {"world_name": worlds[address]}
		if not worlds.is_empty():
			online_locations_changed.emit()

	_online_locations_in_progress = false


## Resolves a World's place title once for ALL rows in that world, caching the result in
## `known_worlds` and re-emitting `online_locations_changed` so listening rows swap their
## fallback (trimmed ENS) label for the real title. Deduped three ways: skips if already
## cached, if a request is in flight, or if the last lookup failed within the cooldown — so an
## unresolvable world can't re-fire a request per row on every emit.
func async_resolve_world_title(world: String) -> void:
	if world.is_empty() or known_worlds.has(world):
		return
	if _world_title_in_progress.has(world):
		return
	var failed_at: int = int(_world_title_failed_at.get(world, -1))
	if failed_at != -1:
		var elapsed: int = Time.get_ticks_msec() - failed_at
		if elapsed < int(WORLD_TITLE_RETRY_COOLDOWN * 1000.0):
			return
	_world_title_in_progress[world] = true

	var result = await PlacesHelper.async_get_by_names(world)
	_world_title_in_progress.erase(world)

	var title: String = ""
	if result != null and not (result is PromiseError):
		var json: Dictionary = result.get_string_response_as_json()
		if not json.data.is_empty():
			title = json.data[0].get("title", "")

	if title.is_empty():
		# Lookup failed (or the place has no title): don't poison known_worlds with a fallback —
		# record the failure so we retry only after the cooldown; rows keep the trimmed ENS.
		_world_title_failed_at[world] = Time.get_ticks_msec()
		return

	_world_title_failed_at.erase(world)
	known_worlds[world] = title
	online_locations_changed.emit()


## Fetches archipelago peers and returns the raw peers array. Owns its own HTTPRequest so it never
## races with the shared `fetch_peers()` + `in_genesis_city_changed` signal path.
func _async_fetch_peers_raw() -> Array:
	var http_request: HTTPRequest = HTTPRequest.new()
	http_request.timeout = 10.0  # never let the await (and _online_locations_in_progress) latch
	add_child(http_request)
	var error: int = http_request.request(DclUrls.archipelago_stats() + "/comms/peers")
	if error != OK:
		http_request.queue_free()
		return []
	var response: Array = await http_request.request_completed
	http_request.queue_free()
	# response = [result, response_code, headers, body]
	if response[0] != HTTPRequest.RESULT_SUCCESS or int(response[1]) != 200:
		return []
	var json: JSON = JSON.new()
	if json.parse((response[3] as PackedByteArray).get_string_from_utf8()) != OK:
		return []
	var data = json.get_data()
	if not (data is Dictionary) or not data.has("peers"):
		return []
	return data["peers"]


## Resolves the connected world for each address (8 concurrent, 15s budget). Returns
## {address_lower -> world_name} only for addresses actually in a world.
func _async_resolve_worlds(addresses: Array) -> Dictionary:
	var world_result: Dictionary = {}
	if addresses.is_empty():
		return world_result

	var base_url: String = _get_worlds_base_url()
	var responses: Dictionary = {}  # address -> data Dictionary or null
	var queue_index: int = 0

	# Launch first batch.
	while queue_index < addresses.size() and queue_index < MAX_CONCURRENT_WORLD_REQUESTS:
		_launch_world_request(base_url, addresses[queue_index], responses)
		queue_index += 1

	# Drain, launching more as slots free up, until all responded or the budget runs out.
	var deadline: int = Time.get_ticks_msec() + int(WORLD_BATCH_TIMEOUT * 1000.0)
	while responses.size() < addresses.size():
		if Time.get_ticks_msec() >= deadline or not is_inside_tree():
			break
		await get_tree().process_frame
		while (
			queue_index < addresses.size()
			and (queue_index - responses.size()) < MAX_CONCURRENT_WORLD_REQUESTS
		):
			_launch_world_request(base_url, addresses[queue_index], responses)
			queue_index += 1

	for address in responses.keys():
		var data = responses[address]
		if data == null or not (data is Dictionary):
			continue
		var world_name: String = data.get("world", "")
		if not world_name.is_empty():
			world_result[address] = world_name
	return world_result


func _launch_world_request(base_url: String, address: String, responses: Dictionary) -> void:
	var http_request: HTTPRequest = HTTPRequest.new()
	http_request.timeout = 10.0  # so a stuck world endpoint frees the node instead of leaking
	add_child(http_request)
	var url: String = base_url + "/wallet/" + str(address).uri_encode() + "/connected-world"
	http_request.request_completed.connect(
		_on_world_request_completed.bind(address, http_request, responses)
	)
	var error: int = http_request.request(url)
	if error != OK:
		http_request.queue_free()
		responses[address] = null


func _on_world_request_completed(
	result: int,
	response_code: int,
	_headers: PackedStringArray,
	body: PackedByteArray,
	address: String,
	http_request: HTTPRequest,
	responses: Dictionary
) -> void:
	http_request.queue_free()
	if result != HTTPRequest.RESULT_SUCCESS or response_code != 200:
		responses[address] = null
		return
	var json: JSON = JSON.new()
	if json.parse(body.get_string_from_utf8()) != OK:
		responses[address] = null
		return
	var data = json.get_data()
	if data is Dictionary:
		responses[address] = data
	else:
		responses[address] = null


func _get_worlds_base_url() -> String:
	# worlds_content_server() returns the ".../world/" content path; the connected-world
	# endpoint lives on the host root, so strip that suffix.
	var url: String = DclUrls.worlds_content_server()
	var idx: int = url.find("/world/")
	if idx >= 0:
		return url.substr(0, idx)
	return url.trim_suffix("/")
