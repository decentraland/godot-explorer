extends CarrouselGenerator

enum OrderBy {
	NONE,
	MOST_ACTIVE,
	LIKE_SCORE,
}

const DISCOVER_CARROUSEL_ITEM = preload(
	"res://src/ui/pages/discover/carrousel/place_discover_card.tscn"
)

const JUMP_BACK_IN_MAX_ITEMS: int = 6
const NEW_IN_DECENTRALAND_PAGE_SIZE: int = 100
const NEW_IN_DECENTRALAND_MAX_AGE_SECONDS: int = 7 * 24 * 60 * 60

@export var order_by: OrderBy = OrderBy.NONE
@export var categories: String = "all"
@export var only_favorites: bool = false
@export var only_highlighted: bool = false
@export var only_featured: bool = false
@export var only_my_places: bool = false
@export var only_worlds: bool = false
@export var last_places_logic: bool = false
@export var new_in_decentraland_logic: bool = false

var _loaded_elements: int = 0
var _no_more_elements: bool = false
var _loading: bool = false
var _discover_carrousel_item_loading: Control = null


func _ready() -> void:
	if only_favorites:
		Global.favorite_destination_set.connect(_on_favorite_changed)


func on_request(offset: int, limit: int) -> void:
	if _no_more_elements and not _new_search:
		return  # we reach the capacity...

	if _new_search:
		clean_items()
		offset = 0

	if last_places_logic:
		async_request_last_places(offset, limit)
	elif new_in_decentraland_logic:
		async_request_new_in_decentraland(offset, limit)
	else:
		async_request_from_api(offset, limit)


func _on_favorite_changed() -> void:
	_new_search = true
	_no_more_elements = false
	async_request_from_api(0, 10)


func clean_items():
	for item in item_container.get_children():
		if item is PlaceItem:
			item_container.remove_child(item)
			item.queue_free()


func async_request_last_places(_offset: int, _limit: int) -> void:
	_no_more_elements = true
	if _loading:
		return

	_loading = true

	var last_places: Array[Dictionary] = Global.get_config().get_last_places()
	last_places = last_places.slice(0, JUMP_BACK_IN_MAX_ITEMS)
	var seen: Dictionary = {}
	var places_data: Array[Dictionary] = []
	var older_visits: Array[Dictionary] = []
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
			# Only a real place id proves it is the same place, not the fallback keys.
			if data.has("id"):
				older_visits.append(place)
			continue
		if not dedup_key.is_empty():
			seen[dedup_key] = true
		places_data.append(data)

	if not older_visits.is_empty():
		Global.get_config().remove_last_places(older_visits)

	# The loop awaits, so the page can go away mid-load.
	if not can_populate():
		_loading = false
		return

	# Discover's re-show calls this directly, without on_request's clean_items(): start from an
	# empty row or every re-show appends a second copy of each card (#3071).
	clean_items()
	for place_data in places_data:
		var item = DISCOVER_CARROUSEL_ITEM.instantiate()
		item_container.add_child(item)
		item.set_data(place_data)
		# Relay through the generator's own signal so a host without a `discover` reference (e.g.
		# the landscape DiscoverPanel) can still react to a tap; `discover`, when set, keeps
		# routing straight to the full Discover screen's own SidePanelWrapper flow.
		item.item_pressed.connect(item_pressed.emit)
		if is_instance_valid(discover):
			item.item_pressed.connect(discover.on_item_pressed.bind(item))

	if places_data.size() > 0:
		report_loading_status.emit(CarrouselGenerator.LoadingStatus.OK_WITH_RESULTS)
	else:
		report_loading_status.emit(CarrouselGenerator.LoadingStatus.OK_WITHOUT_RESULTS)
	_loading = false


## New = created or redeployed in the last 7 days. The API sorts by one column and has no
## deployed_at order, so this joins the newest destinations (created_at, paged while the window
## lasts) with the most active page filtered on deployed_at, which holds every destination with
## people in it. Redeployed scenes nobody is in are left out.
func async_request_new_in_decentraland(_offset: int, _limit: int) -> void:
	_no_more_elements = true
	if _loading:
		return

	_loading = true
	# Cleared so on_request's guard holds once the window is shown: scrolling past card 10 would
	# otherwise clean the row and fetch it again.
	_new_search = false
	report_loading_status.emit(CarrouselGenerator.LoadingStatus.LOADING)

	var now: int = int(Time.get_unix_time_from_system())
	var candidates: Array = []
	var seen_ids: Dictionary = {}
	var page_offset: int = 0
	var window_ended: bool = false
	while not window_ended:
		var page = await _async_fetch_new_in_decentraland_page(
			"offset=%d&order_by=created_at&order=desc" % page_offset
		)
		if not page is Array:
			return
		# Filtered item by item: with tag=allowed_ios the BFF returns places then worlds, each
		# sorted on its own, so a stale row does not mean the rest are stale too.
		for item_data in page:
			if _is_within_new_window(item_data, "created_at", now):
				seen_ids[item_data.get("id")] = true
				candidates.append(item_data)

		# Not `+= PAGE_SIZE`: with tag=allowed_ios the BFF ignores `limit` and returns everything
		# from the offset on.
		if (
			page.size() < NEW_IN_DECENTRALAND_PAGE_SIZE
			or not _is_within_new_window(page.back(), "created_at", now)
		):
			window_ended = true
		page_offset += page.size()

	var most_active = await _async_fetch_new_in_decentraland_page("offset=0&order_by=most_active")
	if not most_active is Array:
		return
	for item_data in most_active:
		if seen_ids.has(item_data.get("id")):
			continue
		if _is_within_new_window(item_data, "deployed_at", now):
			candidates.append(item_data)

	candidates.sort_custom(_sort_by_active_users_then_rating)

	for item_data in candidates:
		var item = DISCOVER_CARROUSEL_ITEM.instantiate()
		item_container.add_child(item)
		item.set_data(item_data)
		item.item_pressed.connect(item_pressed.emit)
		if is_instance_valid(discover):
			item.item_pressed.connect(discover.on_item_pressed.bind(item))

	if candidates.size() > 0:
		report_loading_status.emit(CarrouselGenerator.LoadingStatus.OK_WITH_RESULTS)
	else:
		report_loading_status.emit(CarrouselGenerator.LoadingStatus.OK_WITHOUT_RESULTS)
	_loading = false


## Returns the page's destinations, or null once it has reported an error or can no longer
## populate.
func _async_fetch_new_in_decentraland_page(params: String) -> Variant:
	var url: String = (
		PlacesHelper.get_api_url() + "?%s&limit=%d&sdk=7" % [params, NEW_IN_DECENTRALAND_PAGE_SIZE]
	)
	if Global.is_ios_or_emulating():
		url += "&tag=allowed_ios"

	var response = await Global.async_signed_fetch(url, HTTPClient.METHOD_GET, "")
	if not can_populate():
		_loading = false
		return null

	if response is PromiseError:
		printerr("Error request places ", url, " ", response.get_error())
		report_loading_status.emit(CarrouselGenerator.LoadingStatus.ERROR)
		_loading = false
		return null

	return response.get_string_response_as_json().data


func _is_within_new_window(item_data: Dictionary, field: String, now: int) -> bool:
	var timestamp = item_data.get(field)
	if not timestamp is String or timestamp.is_empty():
		return false
	var unix: int = Time.get_unix_time_from_datetime_string(timestamp.substr(0, 19))
	return now - unix <= NEW_IN_DECENTRALAND_MAX_AGE_SECONDS


func _sort_by_active_users_then_rating(a: Dictionary, b: Dictionary) -> bool:
	var a_users: int = a.get("user_count", 0)
	var b_users: int = b.get("user_count", 0)
	if a_users != b_users:
		return a_users > b_users
	var a_rating = a.get("like_score")
	var b_rating = b.get("like_score")
	return (a_rating if a_rating != null else 0.0) > (b_rating if b_rating != null else 0.0)


func async_request_from_api(offset: int, limit: int) -> void:
	var query := "?offset=%d&limit=%d" % [offset, limit]

	if only_worlds:
		query += "&only_worlds=true"

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
				load("res://src/ui/pages/discover/carrousel/discover_carrousel_item_loading.tscn")
				. instantiate()
			)
			item_container.add_child(_discover_carrousel_item_loading)

		item_container.move_child(_discover_carrousel_item_loading, -1)

	if search_param.length() > 0:
		query += "&search=" + search_param.uri_encode()

	if only_favorites:
		query += "&only_favorites=true"

	if only_my_places:
		var address := Global.player_identity.get_address_str()
		if not address.is_empty():
			query += "&owner=" + address

	if only_highlighted:
		query += "&only_highlighted=true"

	if only_featured:
		query += "&tag=featured"

	if order_by != OrderBy.NONE:
		query += "&order_by=" + ("like_score" if order_by == OrderBy.LIKE_SCORE else "most_active")

	if Global.is_ios_or_emulating():
		query += "&tag=allowed_ios"

	query += "&sdk=7"

	if categories != "all":
		var categories_array = categories.split(",")
		for category in categories_array:
			query += "&categories=" + category

	var base_url := (
		PlacesHelper.get_sign_api_url() if only_favorites else PlacesHelper.get_api_url()
	)
	var url := base_url + query
	_async_fetch_places(url, limit)


func _async_fetch_places(url: String, limit: int = 100) -> void:
	var response = await Global.async_signed_fetch(url, HTTPClient.METHOD_GET, "")

	if not can_populate():
		return

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
		# Relay through the generator's own signal so a host without a `discover` reference (e.g.
		# the landscape DiscoverPanel) can still react to a tap; `discover`, when set, keeps
		# routing straight to the full Discover screen's own SidePanelWrapper flow.
		item.item_pressed.connect(item_pressed.emit)
		if is_instance_valid(discover):
			item.item_pressed.connect(discover.on_item_pressed.bind(item))

	report_loading_status.emit(CarrouselGenerator.LoadingStatus.OK_WITH_RESULTS)
