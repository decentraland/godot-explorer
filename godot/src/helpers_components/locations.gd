extends Node

signal in_genesis_city_changed(players: Array)

var known_locations: Array = []  # Array of objects {coord: [x,y], title: String}
var in_genesis_city: Array = []  # Array of objects {address: String, parcel: [int, int]}


func fetch_peers():
	var http_request = HTTPRequest.new()
	add_child(http_request)
	http_request.request_completed.connect(_on_request_completed.bind(http_request))

	var error = http_request.request("https://archipelago-ea-stats.decentraland.org/comms/peers")
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
		return

	if response_code != 200:
		push_error("Error in response code: " + str(response_code))
		return

	# Parse the JSON
	var response = body.get_string_from_utf8()
	var json = JSON.new()
	var parse_result = json.parse(response)

	if parse_result != OK:
		push_error("Error parsing JSON: " + json.get_error_message())
		return

	var data = json.get_data()
	if not data.has("peers"):
		push_error("Response does not contain 'peers'")
		return

	# Create the online_players array
	in_genesis_city.clear()
	var peers = data["peers"]

	for peer in peers:
		if peer.has("address") and peer.has("parcel"):
			var player = {
				"address": str(peer["address"]),
				"parcel": [int(peer["parcel"][0]), int(peer["parcel"][1])]
			}
			in_genesis_city.append(player)

	# Emit signal when online_players changes
	in_genesis_city_changed.emit(in_genesis_city)
