class_name PlacesHelper

static func async_post_like(place_id, toggled_on : bool) -> Variant:
	var url = DclUrls.places_api() + "/places/" + place_id + "/likes"
	var body

	if toggled_on:
		body = JSON.stringify({like = false})
	else:
		body = JSON.stringify({like = null})

	return await Global.async_signed_fetch(url, HTTPClient.METHOD_PATCH, body)


static func async_post_favorite(place_id: String, toggled_on : bool) -> Variant:
	var url = DclUrls.places_api() + "/places/" + place_id + "/favorites"
	var body

	if toggled_on:
		body = JSON.stringify({like = false})
	else:
		body = JSON.stringify({like = null})

	return await Global.async_signed_fetch(url, HTTPClient.METHOD_PATCH, body)


static func async_get_by_position(pos: Vector2i) -> Variant:

	var url: String = DclUrls.places_api() + "/places?limit=1"
	url += "&positions=%d,%d" % [pos.x, pos.y]

	var headers = {"Content-Type": "application/json"}
	var promise: Promise = Global.http_requester.request_json(
		url, HTTPClient.METHOD_GET, "", headers
	)
	return await PromiseUtils.async_awaiter(promise)


static func async_get_by_id(place_id: String) -> Variant:
	var url: String = DclUrls.places_api() + "/places/" + place_id

	#var headers = {"Content-Type": "application/json"}
	#var promise: Promise = Global.http_requester.request_json(
	#	url, HTTPClient.METHOD_GET, "", headers
	#)
	#return await PromiseUtils.async_awaiter(promise)
	return await Global.async_signed_fetch(url, HTTPClient.METHOD_GET)
