extends CarrouselGenerator

enum OrderBy {
	NONE,
	MOST_ACTIVE,
	LIKE_SCORE,
}

const DISCOVER_CARROUSEL_ITEM = preload(
	"res://src/ui/components/discover/carrousel/discover_carrousel_item.tscn"
)

@export var order_by: OrderBy = OrderBy.NONE
@export var categories: String = "all"
@export var only_favorites: bool = false
@export var only_highlighted: bool = false
@export var only_worlds: bool = false
@export var last_places_logic: bool = false

var loaded_elements: int = 0
var no_more_elements: bool = false


func _request_last_places() -> void:
	var last_places = Global.config.last_places.duplicate()
	var genesis_city_places: Array[Dictionary] = []
	var worlds_names: Array[Dictionary] = []
	var custom_realms: Array[Dictionary] = []
	var index = 0

	(
		last_places
		. push_front(
			{
				"realm": Global.config.last_realm_joined,
				"position": Global.config.last_parcel_position,
			}
		)
	)

	for place in last_places:
		Global.config.fix_last_places_duplicates(place, last_places)

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

		(
			item
			. set_data(
				{
					"title": realm,
					"base_position": "%d,%d" % [position.x, position.y],
					"world": true,
					"world_name": realm,
				}
			)
		)
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


func on_request(offset: int, limit: int) -> void:
	if no_more_elements:
		return  # we reach the capacity...

	if last_places_logic:
		no_more_elements = true
		_request_last_places()
		return

	var url = "https://places.decentraland.org/api/"
	url += "worlds" if only_worlds else "places"

	url += "?offset=%d&limit=%d" % [offset, limit]

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

	if result is PromiseError:
		set_consumer_visible.emit(false)
		printerr("Error request places", result.get_error())
		return

	var json: Dictionary = result.get_string_response_as_json()

	if json.data.is_empty():
		if loaded_elements == 0:
			set_consumer_visible.emit(false)
		return

	loaded_elements += json.data.size()

	if json.data.size() != limit:
		no_more_elements = true

	for item_data in json.data:
		var item = DISCOVER_CARROUSEL_ITEM.instantiate()
		item_container.add_child(item)

		item.set_data(item_data)
		item.item_pressed.connect(discover.on_item_pressed)

	set_consumer_visible.emit(true)
