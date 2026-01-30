class_name PlacesHelper

enum LIKE { UNKNOWN, YES, NO }

## Temp constant to test the new Destinations endpoint.
## Currently not 100% functional
const USE_DESTINATIONS := false


static func get_api_url() -> String:
	if USE_DESTINATIONS:
		return "https://places.decentraland.zone/api/destinations/"
	return DclUrls.places_api() + "/places/"


static func async_patch_like(place_id: String, like: LIKE) -> Variant:
	var url := get_api_url() + place_id + "/likes"

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
	var url := get_api_url() + place_id + "/favorites"

	var body: String
	if toggled_on:
		body = JSON.stringify({favorites = true})
	else:
		body = JSON.stringify({favorites = false})

	var respnse = await Global.async_signed_fetch(url, HTTPClient.METHOD_PATCH, body)

	Global.favorite_destination_set.emit()

	return respnse


static func async_get_by_position(pos: Vector2i) -> Variant:
	var url: String = get_api_url() + "?limit=1"
	url += "&positions=%d,%d" % [pos.x, pos.y]

	var headers = {"Content-Type": "application/json"}
	var promise: Promise = Global.http_requester.request_json(
		url, HTTPClient.METHOD_GET, "", headers
	)
	return await PromiseUtils.async_awaiter(promise)


static func async_get_by_names(name: String) -> Variant:
	var url: String
	if USE_DESTINATIONS:
		#url = get_api_url() + "?world_names=%s&limit=1&only_worlds=true" % name.uri_encode()
		url = get_api_url() + "?names=%s&only_worlds=true&limit=1" % name.uri_encode()
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
