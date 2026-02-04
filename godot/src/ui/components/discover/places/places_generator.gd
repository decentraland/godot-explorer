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
@export var only_featured: bool = false
@export var only_my_places: bool = false
@export var only_worlds: bool = false
@export var last_places_logic: bool = false

var _loaded_elements: int = 0
var _no_more_elements: bool = false
var _loading: bool = false
var _discover_carrousel_item_loading: Control = null


func on_request(offset: int, limit: int) -> void:
	if _no_more_elements and not _new_search:
		return  # we reach the capacity...

	if last_places_logic:
		async_request_last_places(offset, limit)
	else:
		async_request_from_api(offset, limit)

	if only_favorites:
		Global.favorite_destination_set.connect(reload.bind(offset, limit))


func reload(offset, limit) -> void:
	_new_search = true
	async_request_from_api(offset, limit)


func async_request_last_places(_offset: int, _limit: int) -> void:
	_no_more_elements = true
	if _loading:
		return

	for item in item_container.get_children():
		if item is PlaceItem:
			item_container.remove_child(item)
			item.queue_free()

	_loading = true

	var last_places: Array[Dictionary] = Global.get_config().last_places.duplicate()
	var seen: Dictionary = {}
	var index = 0
	for place in last_places:
		place["index"] = index
		index += 1

		var realm: String = Realm.ensure_reduce_url(place.get("realm"))
		var position: Vector2i = place.get("position")
		var data: Dictionary
		var response

		if Realm.is_genesis_city(realm):
			response = await PlacesHelper.async_get_by_position(Vector2i(place.position))
		else:
			response = await PlacesHelper.async_get_by_names(place.get("realm"))

		if response:
			if response is PromiseError:
				printerr("Error request places ", place, " ", response.get_error())
				continue
			var json: Dictionary = response.get_string_response_as_json()
			if json.data.is_empty():
				continue
			data = json.data[0]
		else:
			data = {
				"title": realm,
				"world": true,
				"world_name": realm,
				"base_position": "%d,%d" % [position.x, position.y]
			}

		var dedup_key: String = data.get("id", data.get("base_position", ""))
		if dedup_key.is_empty():
			dedup_key = data.get("world_name", "")
		if not dedup_key.is_empty() and seen.has(dedup_key):
			continue
		if not dedup_key.is_empty():
			seen[dedup_key] = true

		var item = DISCOVER_CARROUSEL_ITEM.instantiate()
		item_container.add_child(item)
		item.set_data(data)
		item.item_pressed.connect(discover.on_item_pressed)

	if last_places.size() > 0:
		report_loading_status.emit(CarrouselGenerator.LoadingStatus.OK_WITH_RESULTS)

	_loading = false


func async_request_from_api(offset: int, limit: int) -> void:
	var url: String = PlacesHelper.get_api_url()
	url += "?offset=%d&limit=%d" % [offset, limit]
	if only_worlds:
		url += "&only_worlds=true"

	if _new_search:
		_loaded_elements = 0
		_new_search = false
		report_loading_status.emit(CarrouselGenerator.LoadingStatus.LOADING)

		if is_instance_valid(item_container):
			for child in item_container.get_children():
				child.queue_free()
				item_container.remove_child(child)
	else:
		if is_instance_valid(_discover_carrousel_item_loading):
			_discover_carrousel_item_loading.show()
		else:
			_discover_carrousel_item_loading = (
				load(
					"res://src/ui/components/discover/carrousel/discover_carrousel_item_loading.tscn"
				)
				. instantiate()
			)
			item_container.add_child(_discover_carrousel_item_loading)

		item_container.move_child(_discover_carrousel_item_loading, -1)

	if search_param.length() > 0:
		url += "&search=" + search_param.uri_encode()

	if only_favorites:
		url += "&only_favorites=true"

	if only_my_places:
		var address := Global.player_identity.get_address_str()
		if not address.is_empty():
			url += "&owner=" + address

	if only_highlighted:
		url += "&only_highlighted=true"

	if only_featured:
		url += "&tag=featured"

	if order_by != OrderBy.NONE:
		url += "&order_by=" + ("like_score" if order_by == OrderBy.LIKE_SCORE else "most_active")

	if Global.is_ios_or_emulating():
		url += "&tag=allowed_ios"

	url += "&sdk=7"

	if categories != "all":
		var categories_array = categories.split(",")
		for category in categories_array:
			url += "&categories=" + category

	_async_fetch_places(url, limit)



func _async_fetch_places(url: String, limit: int = 100) -> void:
	var response = await Global.async_signed_fetch(url, HTTPClient.METHOD_GET, "")

	if is_instance_valid(_discover_carrousel_item_loading):
		_discover_carrousel_item_loading.hide()

	if response is PromiseError:
		if _loaded_elements == 0:
			report_loading_status.emit(CarrouselGenerator.LoadingStatus.ERROR)
		printerr("Error request places ", url, " ", response.get_error())
		return

	var json: Dictionary = response.get_string_response_as_json()

	_no_more_elements = json.data.size() != limit

	if json.data.is_empty():
		if _loaded_elements == 0 and _no_more_elements:
			report_loading_status.emit(CarrouselGenerator.LoadingStatus.OK_WITHOUT_RESULTS)
		return

	_loaded_elements += json.data.size()

	for item_data in json.data:
		var item := DISCOVER_CARROUSEL_ITEM.instantiate()
		item_container.add_child(item)

		item.set_data(item_data)
		item.item_pressed.connect(discover.on_item_pressed)

	report_loading_status.emit(CarrouselGenerator.LoadingStatus.OK_WITH_RESULTS)
