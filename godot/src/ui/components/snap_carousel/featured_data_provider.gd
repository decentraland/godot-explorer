class_name FeaturedDataProvider

## Fetches featured scenes for the FTUE carousel.
## Tries the dedicated mobile-bff endpoint first, falls back to destinations API.

const FALLBACK_LIMIT: int = 3


static func async_fetch_ftue_places() -> Array[Dictionary]:
	# Try dedicated endpoint first
	var places := await _async_fetch_from_bff()
	if places.is_empty():
		# Fallback to destinations API featured places
		places = await _async_fetch_from_destinations()
	places.shuffle()
	return places


static func _async_fetch_from_bff() -> Array[Dictionary]:
	var url := DclUrls.mobile_bff() + "/discover-featured/scenes"
	var response = await Global.async_signed_fetch(url, HTTPClient.METHOD_GET, "")
	if response is PromiseError:
		printerr("[FeaturedDataProvider] BFF endpoint failed:", response.get_error())
		return []

	var json = response.get_string_response_as_json()
	var places: Array[Dictionary] = []
	var data: Array = json if json is Array else json.get("data", [])
	for item in data:
		if item is Dictionary:
			places.append(item)
	return places


static func _async_fetch_from_destinations() -> Array[Dictionary]:
	var url := PlacesHelper.get_api_url() + "?offset=0&limit=%d&tag=featured&sdk=7" % FALLBACK_LIMIT
	if Global.is_ios_or_emulating():
		url += "&tag=allowed_ios"

	var response = await Global.async_signed_fetch(url, HTTPClient.METHOD_GET, "")
	if response is PromiseError:
		printerr("[FeaturedDataProvider] Fallback fetch failed:", response.get_error())
		return []

	var json: Dictionary = response.get_string_response_as_json()
	var places: Array[Dictionary] = []
	for item in json.get("data", []):
		places.append(item)
	return places
