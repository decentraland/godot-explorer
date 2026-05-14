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
