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

		var item = DISCOVER_CARROUSEL_ITEM.instantiate()
		item_container.add_child(item)
		item.set_data(data)
		item.item_pressed.connect(discover.on_item_pressed)

	if last_places.size() > 0:
		report_loading_status.emit(CarrouselGenerator.LoadingStatus.OK_WITH_RESULTS)

	_loading = false


func async_request_from_api(offset: int, limit: int) -> void:
	await IosAllowedList.async_ensure_loaded()

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

	#TODO Authorization required?
	if only_favorites:
		url += "&only_favorites=true"

	# iOS BFF-driven filtering for Featured and Most Active carousels
	var is_ios := Global.is_ios_or_emulating()
	var use_bff_featured := is_ios and (only_highlighted or only_featured)
	var use_bff_most_active := (
		is_ios
		and order_by == OrderBy.MOST_ACTIVE
		and not only_highlighted
		and not only_featured
		and not only_favorites
	)

	if use_bff_featured:
		var bff_url := DclUrls.mobile_bff() + "/destinations/?tag=allowed_ios&tag=featured"
		var bff_positions := await IosAllowedList.async_fetch_bff_positions(bff_url)
		url += bff_positions
	elif use_bff_most_active:
		var bff_url := DclUrls.mobile_bff() + "/destinations/?tag=allowed_ios&orderBy=mostActive"
		var bff_positions := await IosAllowedList.async_fetch_bff_positions(bff_url)
		url += bff_positions
		url += "&order_by=most_active"
	else:
		if only_highlighted:
			url += "&only_highlighted=true"

		if order_by != OrderBy.NONE:
			url += (
				"&order_by=" + ("like_score" if order_by == OrderBy.LIKE_SCORE else "most_active")
			)

		if only_worlds:
			url += IosAllowedList.get_names_query_params()
		else:
			url += IosAllowedList.get_positions_query_params()

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
