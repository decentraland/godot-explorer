extends CarrouselGenerator

const EVENT_DISCOVER_CARD = preload("res://src/ui/components/events/event_discover_card.tscn")

@export var only_trending: bool = false
@export var only_live: bool = false

var loaded_elements: int = 0
var no_more_elements: bool = false
var loading = false
var discover_carrousel_item_loading: Control = null


func on_request(_offset: int, limit: int) -> void:
	if no_more_elements and not new_search:
		return  # we reach the capacity...

	if new_search:
		loaded_elements = 0
		new_search = false
		report_loading_status.emit(CarrouselGenerator.LoadingStatus.LOADING)

		if is_instance_valid(item_container):
			for child in item_container.get_children():
				child.queue_free()
				item_container.remove_child(child)
	else:
		if is_instance_valid(discover_carrousel_item_loading):
			discover_carrousel_item_loading.show()
		else:
			discover_carrousel_item_loading = (
				load(
					"res://src/ui/components/discover/carrousel/discover_carrousel_item_loading.tscn"
				)
				. instantiate()
			)
			item_container.add_child(discover_carrousel_item_loading)

		item_container.move_child(discover_carrousel_item_loading, -1)

	# TODO: Implement more filters (categories, sorting, etc.)
	# For now we only query the events API URL
	var url = DclUrls.events_api() + "/events/"
	if search_param.length() > 0:
		url += "?search=" + search_param.replace(" ", "%20")
	_async_fetch_events(url, limit)


func _async_fetch_events(url: String, limit: int = 100):
	var response = await Global.async_signed_fetch(url, HTTPClient.METHOD_GET, "")

	if is_instance_valid(discover_carrousel_item_loading):
		discover_carrousel_item_loading.hide()

	if response is PromiseError:
		if loaded_elements == 0:
			report_loading_status.emit(CarrouselGenerator.LoadingStatus.ERROR)
		printerr("Error request events ", url, " ", response.get_error())
		return

	var json: Dictionary = response.get_string_response_as_json()

	if not json.has("data") or json.data.is_empty():
		if loaded_elements == 0:
			report_loading_status.emit(CarrouselGenerator.LoadingStatus.OK_WITHOUT_RESULTS)
		return

	loaded_elements += json.data.size()

	if json.data.size() != limit:
		no_more_elements = true

	var filtered_data = []
	for event_data in json.data:
		if event_data.get("approved", false) == true:
			filtered_data.append(event_data)
		json.data = filtered_data

	if only_trending:
		for event_data in json.data:
			if event_data.get("trending", false) == true:
				filtered_data.append(event_data)
		json.data = filtered_data

	if json.data.is_empty():
		if loaded_elements == 0:
			report_loading_status.emit(CarrouselGenerator.LoadingStatus.OK_WITHOUT_RESULTS)
		return

	_sort_events_by_priority(json.data)

	for event_data in json.data:
		var item = EVENT_DISCOVER_CARD.instantiate()
		item_container.add_child(item)

		item.set_data(event_data)
		item.event_pressed.connect(discover._async_handle_event_notification)

	report_loading_status.emit(CarrouselGenerator.LoadingStatus.OK_WITH_RESULTS)


func _sort_events_by_priority(events_array: Array) -> void:
	"""
	sorting criteria:
	1. live = true and trending = true
	2. live = true
	3. live = false ordered by next_start_at
	"""
	events_array.sort_custom(_compare_events_priority)


# gdlint:disable=max-returns
func _compare_events_priority(a: Dictionary, b: Dictionary) -> bool:
	var a_live = a.get("live", false)
	var b_live = b.get("live", false)
	var a_trending = a.get("trending", false)
	var b_trending = b.get("trending", false)

	var a_priority1 = a_live and a_trending
	var b_priority1 = b_live and b_trending

	if a_priority1 and not b_priority1:
		return true
	if not a_priority1 and b_priority1:
		return false

	if a_priority1 and b_priority1:
		return false

	var a_priority2 = a_live and not a_trending
	var b_priority2 = b_live and not b_trending

	if a_priority2 and not b_priority2:
		return true
	if not a_priority2 and b_priority2:
		return false

	if a_priority2 and b_priority2:
		return false

	if not a_live and not b_live:
		var a_next_start = a.get("next_start_at", "")
		var b_next_start = b.get("next_start_at", "")

		if a_next_start != "" and b_next_start != "":
			return a_next_start < b_next_start

		if a_next_start != "" and b_next_start == "":
			return true
		if a_next_start == "" and b_next_start != "":
			return false

		return false

	if a_live and not b_live:
		return true
	if not a_live and b_live:
		return false

	return false
# gdlint:enable=too-many-returns
