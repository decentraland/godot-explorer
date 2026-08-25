class_name FeaturedDataProvider

## Fetches places from the destinations API filtered by tag.


static func async_fetch_places(tag: String) -> Array[Dictionary]:
	var url = PlacesHelper.get_api_url() + "?offset=0&tag=%s&sdk=7" % tag
	if Global.is_ios_or_emulating():
		url += "&tag=allowed_ios"

	var response = await Global.async_signed_fetch(url, HTTPClient.METHOD_GET, "")
	if response is PromiseError:
		printerr("[FeaturedDataProvider] Fetch failed (tag=%s): " % tag, response.get_error())
		return []

	var json = response.get_string_response_as_json()
	if json == null:
		return []
	var places: Array[Dictionary] = []
	var data: Array
	if json is Array:
		data = json
	elif json is Dictionary:
		data = json.get("data", [])
	else:
		return []
	for item in data:
		if item is Dictionary:
			places.append(item)
	places.shuffle()
	return places


## Fetches a curated set of places by id, preserving the given order (issue #2670).
##
## Issued concurrently: this runs on the FTUE, where up to PLACE_IDS_MAX signed fetches in
## series would stack their round-trips on the one screen a new user is waiting on.
## Ids that fail to resolve are skipped rather than aborting the set — a campaign whose
## carousel is half-broken still shows the half that works.
static func async_fetch_places_by_ids(place_ids: Array) -> Array[Dictionary]:
	var promises: Array = []
	for place_id in place_ids:
		var id := String(place_id)
		if not id.is_empty():
			promises.append(_place_promise(id))

	var places: Array[Dictionary] = []
	if promises.is_empty():
		return places

	var results: Array = await PromiseUtils.async_all(promises)
	for place_data in results:
		if typeof(place_data) == TYPE_DICTIONARY and not place_data.is_empty():
			places.append(place_data)
	return places


# Starts the fetch and hands back a promise that always resolves — with the place data, or
# with null for an id that could not be read. async_all keeps the input order.
static func _place_promise(id: String) -> Promise:
	var promise := Promise.new()
	_async_resolve_place(promise, id)
	return promise


# gdlint:ignore = async-function-name
static func _async_resolve_place(promise: Promise, id: String) -> void:
	var response = await PlacesHelper.async_get_place_by_id(id)
	if response is PromiseError:
		printerr("[FeaturedDataProvider] Place fetch failed (id=%s): " % id, response.get_error())
		promise.resolve_with_data(null)
		return

	var json = response.get_string_response_as_json()
	if typeof(json) != TYPE_DICTIONARY:
		promise.resolve_with_data(null)
		return

	var place_data = json.get("data", json)
	if typeof(place_data) == TYPE_ARRAY and not place_data.is_empty():
		place_data = place_data[0]
	if typeof(place_data) != TYPE_DICTIONARY:
		promise.resolve_with_data(null)
		return

	promise.resolve_with_data(place_data)
