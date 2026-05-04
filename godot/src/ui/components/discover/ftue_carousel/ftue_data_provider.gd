class_name FtueDataProvider

## Placeholder place IDs for FTUE carousel.
## Replace with a real endpoint call when available.
const FTUE_PLACE_IDS: Array[String] = [
	"780f04dd-eba1-41a8-b109-74896c87e98b",  # Genesis Plaza
]


static func async_fetch_ftue_places() -> Array[Dictionary]:
	var places: Array[Dictionary] = []
	for place_id in FTUE_PLACE_IDS:
		var response = await PlacesHelper.async_get_place_by_id(place_id)
		if response is PromiseError:
			printerr("[FtueDataProvider] Failed to fetch place: ", place_id)
			continue
		var json: Dictionary = response.get_string_response_as_json()
		var place_data: Dictionary = json.get("data", json)
		if not place_data.is_empty():
			places.append(place_data)
	places.shuffle()
	return places
