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

var _loaded_elements: int = 0
var _no_more_elements: bool = false
var _loading: bool = false
var _discover_carrousel_item_loading: Control = null


func on_request(offset: int, limit: int) -> void:
	if _no_more_elements and not _new_search:
		return  # we reach the capacity...

	if last_places_logic:
		request_last_places(offset, limit)
	else:
		request_from_api(offset, limit)


func request_last_places(offset: int, limit: int) -> void:
	_no_more_elements = true
	if _loading:
		return

	for item in item_container.get_children():
		if item is PlaceItem:
			item_container.remove_child(item)
			item.queue_free()

	_loading = true
	
	var last_places :Array[Dictionary] = Global.get_config().last_places.duplicate()
	var index = 0
	for place in last_places:
		place["index"] = index
		index += 1
		var item = DISCOVER_CARROUSEL_ITEM.instantiate()
		item_container.add_child(item)
		item.set_data(place)
		item.item_pressed.connect(discover.on_item_pressed)
		prints("[CAROUSEL]", place)
	
	if last_places.size() > 0:
		report_loading_status.emit(CarrouselGenerator.LoadingStatus.OK_WITH_RESULTS)


	_loading = false


func request_from_api(offset: int, limit: int) -> void:
	#var url = DclUrls.places_api() + "/"
	#url += "worlds" if only_worlds else "places"

	var url := "https://places.decentraland.zone/api/destinations/"
	url += "?offset=%d&limit=%d" % [offset, limit]

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

	if only_highlighted:
		url += "&only_highlighted=true"

	if order_by != OrderBy.NONE:
		url += "&order_by=" + ("like_score" if order_by == OrderBy.LIKE_SCORE else "most_active")

	if categories != "all":
		var categories_array = categories.split(",")
		for category in categories_array:
			url += "&categories=" + category

	_async_fetch_places(url, limit)


func _async_fetch_places(url: String, limit: int = 100) -> void:
	var response = await Global.async_signed_fetch(url, HTTPClient.METHOD_GET, "")

	if only_highlighted:
		prints("[CAROUSEL]", url)

	if is_instance_valid(_discover_carrousel_item_loading):
		_discover_carrousel_item_loading.hide()

	if response is PromiseError:
		if _loaded_elements == 0:
			report_loading_status.emit(CarrouselGenerator.LoadingStatus.ERROR)
		printerr("Error request places ", url, " ", response.get_error())
		return

	var json: Dictionary = response.get_string_response_as_json()

	if json.data.is_empty():
		if _loaded_elements == 0:
			report_loading_status.emit(CarrouselGenerator.LoadingStatus.OK_WITHOUT_RESULTS)
		return

	_loaded_elements += json.data.size()

	if json.data.size() != limit:
		_no_more_elements = true

	for item_data in json.data:
		var item = DISCOVER_CARROUSEL_ITEM.instantiate()
		item_container.add_child(item)

		item.set_data(item_data)
		item.item_pressed.connect(discover.on_item_pressed)

	report_loading_status.emit(CarrouselGenerator.LoadingStatus.OK_WITH_RESULTS)
