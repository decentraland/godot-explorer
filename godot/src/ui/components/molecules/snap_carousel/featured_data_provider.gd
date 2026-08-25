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
## Ids that fail to resolve are skipped rather than aborting the set — a campaign whose
## carousel is half-broken still shows the half that works.
static func async_fetch_places_by_ids(place_ids: Array) -> Array[Dictionary]:
	var places: Array[Dictionary] = []
	for place_id in place_ids:
		var id := String(place_id)
		if id.is_empty():
			continue
		var response = await PlacesHelper.async_get_place_by_id(id)
		if response is PromiseError:
			printerr(
				"[FeaturedDataProvider] Place fetch failed (id=%s): " % id, response.get_error()
			)
			continue
		var json = response.get_string_response_as_json()
		if typeof(json) != TYPE_DICTIONARY:
			continue
		var place_data = json.get("data", json)
		if typeof(place_data) == TYPE_ARRAY and not place_data.is_empty():
			place_data = place_data[0]
		if typeof(place_data) != TYPE_DICTIONARY or place_data.is_empty():
			continue
		places.append(place_data)
	return places
