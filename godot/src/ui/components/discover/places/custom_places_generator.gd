class_name CustomPlacesGenerator
extends CarrouselGenerator


const DISCOVER_CARROUSEL_ITEM = preload(
	"res://src/ui/components/discover/carrousel/discover_carrousel_item.tscn"
)


func add_item(item_data: Dictionary):
	var item = DISCOVER_CARROUSEL_ITEM.instantiate()
	_item_container.add_child(item)

	item.set_data(item_data)
	item.item_pressed.connect(discover.on_item_pressed)


func on_request(_offset: int, _limit: int) -> void:
	prints ("[CAROUSEL] on_request", _offset, _limit)
	
	var url := "https://places.decentraland.zone/api/destinations"
	#url += "?with_connected_users=true&limit=10"
	#url += "?only_highlighted=true&order_by=most_active&with_connected_users=true&limit=10"
	url += "?only_highlighted=true&order_by=most_active&limit=10"
	_async_fetch_places(url, 1)


func _async_fetch_places(url: String, _limit: int = 1):
	var response = await Global.async_signed_fetch(url, HTTPClient.METHOD_GET, "")
	if response is PromiseError:
		prints ("[CAROUSEL] PromiseError", response.get_error())
		printerr("Error request places ", url, " ", response.get_error())
		return

	var json: Dictionary = response.get_string_response_as_json()

	if json.data.is_empty():
		prints ("[CAROUSEL] json.data.is_empty")
		return

	for item_data in json.data:
		#prints ("[CAROUSEL] item_data", item_data)
		var item = DISCOVER_CARROUSEL_ITEM.instantiate()
		_item_container.add_child(item)

		item.set_data(item_data)
		item.item_pressed.connect(discover.on_item_pressed)

	report_loading_status.emit(CarrouselGenerator.LoadingStatus.OK_WITH_RESULTS)
