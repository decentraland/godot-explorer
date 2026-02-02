class_name PlacesHelper

enum LIKE { UNKNOWN, YES, NO }

enum FetchResultStatus { OK, ERROR }

## Temp constant to test the new Destinations endpoint.
## Currently not 100% functional
const USE_DESTINATIONS := false


class FetchResult:
	var status: FetchResultStatus
	var promise: Promise
	var promise_error: PromiseError
	var result: Array[Dictionary]

	var result_single: Dictionary:
		set(value):
			pass
		get:
			if result.is_empty():
				return {}
			return result[0]

	func _init(status_param: FetchResultStatus, result_param: Array[Dictionary] = []) -> void:
		status = status_param
		result = result_param


static func get_api_url(force_places := false) -> String:
	if USE_DESTINATIONS and !force_places:
		return DclUrls.places_api() + "/destinations/"
	return DclUrls.places_api() + "/places/"


static func async_patch_like(place_id: String, like: LIKE) -> Variant:
	var url := get_api_url(true) + place_id + "/likes"

	var body: String
	match like:
		LIKE.UNKNOWN:
			body = JSON.stringify({like = null})
		LIKE.YES:
			body = JSON.stringify({like = true})
		LIKE.NO:
			body = JSON.stringify({like = false})

	return await Global.async_signed_fetch(url, HTTPClient.METHOD_PATCH, body)


static func async_patch_favorite(place_id: String, toggled_on: bool) -> Variant:
	var url := get_api_url(true) + place_id + "/favorites"

	var body: String
	if toggled_on:
		body = JSON.stringify({favorites = true})
	else:
		body = JSON.stringify({favorites = false})

	var respnse = await Global.async_signed_fetch(url, HTTPClient.METHOD_PATCH, body)

	Global.favorite_destination_set.emit()

	return respnse


static func async_get_by_position(pos: Vector2i) -> Variant:
	var url: String = get_api_url(true) + "?limit=1"
	url += "&positions=%d,%d" % [pos.x, pos.y]

	var headers = {"Content-Type": "application/json"}
	var promise: Promise = Global.http_requester.request_json(
		url, HTTPClient.METHOD_GET, "", headers
	)
	return await PromiseUtils.async_awaiter(promise)


static func async_get_by_names(name: String) -> Variant:
	var url: String
	if USE_DESTINATIONS:
		url = get_api_url() + "?world_names=%s&limit=1&only_worlds=true" % name.uri_encode()
		#url = get_api_url() + "?names=%s&only_worlds=true&limit=1" % name.uri_encode()
	else:
		url = DclUrls.places_api() + "/worlds?limit=1"
		url += "&names=%s" % name.uri_encode()

	var headers = {"Content-Type": "application/json"}
	var promise: Promise = Global.http_requester.request_json(
		url, HTTPClient.METHOD_GET, "", headers
	)
	return await PromiseUtils.async_awaiter(promise)


static func async_get_by_id(place_id: String) -> Variant:
	var url: String = get_api_url() + place_id

	return await Global.async_signed_fetch(url, HTTPClient.METHOD_GET)
	#return await async_fetch_places(url)


static func async_fetch_places(url: String) -> FetchResult:
	var response = await Global.async_signed_fetch(url, HTTPClient.METHOD_GET, "")

	if response is PromiseError:
		var fetch_result := FetchResult.new(FetchResultStatus.ERROR)
		fetch_result.promise_error = response
		return fetch_result

	var json: Dictionary = response.get_string_response_as_json()

	return FetchResult.new(FetchResultStatus.OK, Array(json.data, int(TYPE_DICTIONARY), "", null))


## Given some coordinates finds the name of the place
## at genesis city. Returns an empty String if can't find one
static func async_get_name_from_coordinates(coordinates: Vector2i) -> String:
	var response = await PlacesHelper.async_get_by_position(coordinates)
	if response:
		if response is PromiseError:
			printerr("Error request places ", coordinates, " ", response.get_error())
		else:
			var json: Dictionary = response.get_string_response_as_json()
			if !json.data.is_empty():
				return json.data[0].title
	return ""


# TODO move somewere else
# Using Dictionary for result_vector because
# Dictionaries pass as reference
static func parse_coordinates(text: String, result_vector: Dictionary) -> bool:
	var coord_regex = RegEx.new()
	coord_regex.compile(r"(?<x>-?\d+),(?<y>-?\d+)")
	var is_coordinate := coord_regex.search(text) != null
	if not is_coordinate:
		return false

	var regex_match := coord_regex.search_all(text)
	for m in regex_match:
		for n in m.names:
			match n:
				"x":
					result_vector.x = m.strings[m.names["x"]].to_int()
				"y":
					result_vector.y = m.strings[m.names["y"]].to_int()
	return true
