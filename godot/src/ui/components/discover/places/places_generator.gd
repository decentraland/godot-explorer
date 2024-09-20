extends CarrouselGenerator

enum OrderBy {
	NONE,
	MOST_ACTIVE,
	LIKE_SCORE,
}

const DISCOVER_CARROUSEL_ITEM = preload(
	"res://src/ui/components/discover/carrousel/discover_carrousel_item.tscn"
)

var discover_carrousel_item_loading: Control = null

@export var order_by: OrderBy = OrderBy.NONE
@export var categories: String = "all"
@export var only_favorites: bool = false
@export var only_highlighted: bool = false
@export var only_worlds: bool = false
@export var last_places_logic: bool = false

var loaded_elements: int = 0
var no_more_elements: bool = false
var loading = false


func _place_exists_in_last_places(
	realm: String, position: Vector2i, last_places: Array[Dictionary]
) -> bool:
	for place in last_places:
		var place_realm = place.get("realm")
		var place_position = place.get("position")
		if realm == place_realm:
			if Realm.is_genesis_city(realm):
				if place_position == position:
					return true
			else:
				return true
	return false


func _try_to_add_to_last_places(realm: String, position: Vector2i, last_places: Array[Dictionary]):
	if not _place_exists_in_last_places(realm, position, last_places):
		var last_place = {
			"realm": realm,
			"position": position,
		}
		last_places.push_front(last_place)


func request_last_places() -> void:
	if loading:
		return

	for item in item_container.get_children():
		if item is PlaceItem:
			item_container.remove_child(item)
			item.queue_free()

	loading = true
	var last_places = Global.get_config().last_places.duplicate()
	var genesis_city_places: Array[Dictionary] = []
	var worlds_names: Array[Dictionary] = []
	var custom_realms: Array[Dictionary] = []
	var index = 0

	_try_to_add_to_last_places(
		Global.get_config().last_realm_joined, Global.get_config().last_parcel_position, last_places
	)

	for place in last_places:
		var realm: String = Realm.ensure_reduce_url(place.get("realm"))
		place["index"] = index

		if Realm.is_genesis_city(realm):
			genesis_city_places.push_back(place)
		elif Realm.is_dcl_ens(realm):
			worlds_names.push_back(place)
		else:
			custom_realms.push_back(place)

		index += 1

	for custom_realm in custom_realms:
		var realm: String = custom_realm.get("realm")
		var position: Vector2i = custom_realm.get("position")
		var item = DISCOVER_CARROUSEL_ITEM.instantiate()
		item_container.add_child(item)
		var place = {
			"title": realm,
			"world": true,
			"world_name": realm,
		}

		for custom_place in CustomPlacesGenerator.CUSTOM_PLACES:
			if custom_place.get("world_name", "") == realm:
				place = custom_place.duplicate()
				break

		place["base_position"] = "%d,%d" % [position.x, position.y]
		item.set_data(place)
		item.item_pressed.connect(discover.on_item_pressed)

	if not genesis_city_places.is_empty():
		var url = "https://places.decentraland.org/api/places?limit=%d" % genesis_city_places.size()

		for place in genesis_city_places:
			var position: Vector2i = place.get("position")
			url += "&positions=%d,%d" % [position.x, position.y]

		_async_fetch_places(url)

	if not worlds_names.is_empty():
		var url = "https://places.decentraland.org/api/worlds?limit=%d" % worlds_names.size()

		for place in worlds_names:
			var realm: String = place.get("realm")
			url += "&names=%s" % realm

		_async_fetch_places(url)

	loading = false


func on_request(offset: int, limit: int) -> void:
	if no_more_elements and not new_search:
		return  # we reach the capacity...

	if last_places_logic:
		no_more_elements = true
		request_last_places()
		return

	var url = "https://places.decentraland.org/api/"
	url += "worlds" if only_worlds else "places"

	url += "?offset=%d&limit=%d" % [offset, limit]

	if new_search:
		loaded_elements = 0
		new_search = false
		report_loading_status.emit(CarrouselGenerator.LoadingStatus.Loading)
	else:
		if is_instance_valid(discover_carrousel_item_loading):
			discover_carrousel_item_loading.show()
		else:
			discover_carrousel_item_loading = load("res://src/ui/components/discover/carrousel/discover_carrousel_item_loading.tscn").instantiate()
			item_container.add_child(discover_carrousel_item_loading)
			
		item_container.move_child(discover_carrousel_item_loading, -1)
		discover_carrousel_item_loading
			
	if search_param.length() > 0:
		url += "&search=" + search_param.replace(" ", "%20")

	if only_favorites:
		url += "&only_favorites=true"

	if only_highlighted:
		url += "&only_highlighted=true"

	if order_by != OrderBy.NONE:
		url += "&order_by=" + ("like_score" if order_by == OrderBy.LIKE_SCORE else "most_active")

	if categories != "all":
		var categories_array = categories.split(",")
		for category in categories_array:
			url += "&categories=" + category

	_async_fetch_places(url, limit)


func _async_fetch_places(url: String, limit: int = 100):
	var headers = ["Content-Type: application/json"]
	var promise: Promise = Global.http_requester.request_json(
		url, HTTPClient.METHOD_GET, "", headers
	)
	var result = await PromiseUtils.async_awaiter(promise)

	if is_instance_valid(discover_carrousel_item_loading):
		discover_carrousel_item_loading.hide()
			
	if result is PromiseError:
		report_loading_status.emit(CarrouselGenerator.LoadingStatus.Error)
		printerr("Error request places", result.get_error())
		return

	var json: Dictionary = result.get_string_response_as_json()

	if json.data.is_empty():
		if loaded_elements == 0:
			report_loading_status.emit(CarrouselGenerator.LoadingStatus.OkWithoutResults)
		return

	loaded_elements += json.data.size()

	if json.data.size() != limit:
		no_more_elements = true

	for item_data in json.data:
		var item = DISCOVER_CARROUSEL_ITEM.instantiate()
		item_container.add_child(item)

		item.set_data(item_data)
		item.item_pressed.connect(discover.on_item_pressed)

	report_loading_status.emit(CarrouselGenerator.LoadingStatus.OkWithResults)
