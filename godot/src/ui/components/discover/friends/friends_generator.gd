extends CarrouselGenerator

const FRIEND_DISCOVER_CARD = preload(
	"res://src/ui/components/discover/friends/friend_discover_card.tscn"
)

const REFRESH_INTERVAL: float = 15.0

var _loading: bool = false
var _dirty: bool = false
var _place_cache: Dictionary = {}  # "x,y" -> place_data
var _current_addresses: Dictionary = {}  # address_lower -> card node
var _connected_signals: bool = false
var _refresh_timer: Timer = null
var _debounce_timer: Timer = null
var _first_load: bool = true
var _count_label: Label = null


func _ready() -> void:
	_connect_realtime_signals()
	_refresh_timer = Timer.new()
	_refresh_timer.wait_time = REFRESH_INTERVAL
	_refresh_timer.timeout.connect(_on_refresh_timer)
	add_child(_refresh_timer)
	_debounce_timer = Timer.new()
	_debounce_timer.wait_time = 1.0
	_debounce_timer.one_shot = true
	_debounce_timer.timeout.connect(_on_debounce_timeout)
	add_child(_debounce_timer)


func _connect_realtime_signals() -> void:
	if _connected_signals:
		return
	_connected_signals = true
	Global.social_service.friend_connectivity_updated.connect(_on_friend_connectivity_updated)


func _on_friend_connectivity_updated(_address: String, _status: int) -> void:
	if _loading:
		_dirty = true
		return
	_debounce_timer.start()


func _on_debounce_timeout() -> void:
	on_request(0, 10)


func _on_refresh_timer() -> void:
	if not _loading and is_instance_valid(discover) and discover.is_visible_in_tree():
		on_request(0, 10)


func on_request(_offset: int, _limit: int) -> void:
	_async_on_request(_offset, _limit)


func _async_on_request(_offset: int, _limit: int) -> void:
	if Global.player_identity.is_guest:
		report_loading_status.emit(CarrouselGenerator.LoadingStatus.OK_WITHOUT_RESULTS)
		return

	if _loading:
		return

	_loading = true

	if _first_load:
		report_loading_status.emit(CarrouselGenerator.LoadingStatus.LOADING)

	# Fetch all friends
	var promise = Global.social_service.get_friends(100, 0, -1)
	var timed_out = await _async_await_with_timeout(promise, 10.0)

	if timed_out or promise.is_rejected():
		_loading = false
		if _first_load:
			report_loading_status.emit(CarrouselGenerator.LoadingStatus.OK_WITHOUT_RESULTS)
		return

	var friends = promise.get_data()
	if friends.is_empty():
		_loading = false
		_remove_all()
		report_loading_status.emit(CarrouselGenerator.LoadingStatus.OK_WITHOUT_RESULTS)
		return

	# Build address -> friend data map
	var friends_by_address: Dictionary = {}
	for friend_data in friends:
		var address: String = friend_data["address"].to_lower()
		friends_by_address[address] = friend_data

	var friend_addresses: Array = friends_by_address.keys()

	# Fetch peers from Archipelago to find friends in Genesis City
	Global.locations.fetch_peers()
	await Global.locations.in_genesis_city_changed
	if not is_instance_valid(item_container):
		_loading = false
		return

	var peers = Global.locations.in_genesis_city

	# Build genesis set: address_lower -> friend data with parcel
	var genesis_friends: Dictionary = {}
	for peer in peers:
		var peer_address: String = str(peer["address"]).to_lower()
		if friends_by_address.has(peer_address):
			var friend = friends_by_address[peer_address].duplicate()
			friend["parcel"] = peer["parcel"]
			genesis_friends[peer_address] = friend

	# Friends not in genesis — candidates for worlds
	var not_in_genesis: Array = []
	for address in friend_addresses:
		if not genesis_friends.has(address):
			not_in_genesis.append(address)

	if _first_load:
		# First load: show genesis cards immediately, worlds appear as they arrive
		await _sync_cards(genesis_friends)
		_first_load = false
		_update_status()
		_fetch_connected_worlds_streaming(not_in_genesis, friends_by_address)
	else:
		# Refresh: collect all online friends (genesis + worlds) then diff
		var world_friends := await _async_fetch_connected_worlds(not_in_genesis, friends_by_address)
		var all_online: Dictionary = genesis_friends.duplicate()
		for address in world_friends.keys():
			all_online[address] = world_friends[address]
		await _sync_cards(all_online)
		_update_status()

	_refresh_timer.start()
	_loading = false

	if _dirty:
		_dirty = false
		_debounce_timer.start()


## Syncs displayed cards with the desired set of online friends.
## - Creates cards for new friends.
## - Updates existing cards with fresh place data.
## - Removes cards for friends no longer online.
func _sync_cards(online_friends: Dictionary) -> void:
	# Remove cards for friends no longer online
	var to_remove: Array = []
	for address in _current_addresses.keys():
		if not online_friends.has(address):
			to_remove.append(address)
	for address in to_remove:
		_remove_card(address)

	# Create or update cards
	for address in online_friends.keys():
		var friend = online_friends[address]
		if _current_addresses.has(address):
			await _update_card(address, friend)
		else:
			await _async_create_friend_card(friend)


func _build_place_data(friend: Dictionary) -> Dictionary:
	var place_data: Dictionary = {}
	var world_name: String = friend.get("world_name", "")

	if not world_name.is_empty():
		var place_name = world_name.trim_suffix(".dcl.eth")
		place_data = {
			"title": place_name,
			"world": true,
			"world_name": world_name,
		}
	else:
		var parcel = friend["parcel"]
		var parcel_pos = Vector2i(int(parcel[0]), int(parcel[1]))
		var cache_key := "%d,%d" % [parcel_pos.x, parcel_pos.y]

		if _place_cache.has(cache_key):
			place_data = _place_cache[cache_key].duplicate()
		else:
			var result = await PlacesHelper.async_get_by_position(parcel_pos)
			if not is_instance_valid(item_container):
				return {}
			if result and not (result is PromiseError):
				var json: Dictionary = result.get_string_response_as_json()
				if not json.data.is_empty():
					place_data = json.data[0]
					_place_cache[cache_key] = place_data
					place_data = place_data.duplicate()

		if place_data.is_empty():
			var place_name = "%d, %d" % [parcel_pos.x, parcel_pos.y]
			place_data = {
				"title": place_name,
				"base_position": "%d,%d" % [parcel_pos.x, parcel_pos.y],
			}

	place_data["_friend_name"] = friend.get("name", friend["address"])
	place_data["_friend_address"] = friend["address"]
	place_data["_friend_profile_picture_url"] = friend.get("profile_picture_url", "")
	place_data["_friend_has_claimed_name"] = friend.get("has_claimed_name", false)
	return place_data


func _async_create_friend_card(friend: Dictionary) -> void:
	var place_data := await _build_place_data(friend)
	if place_data.is_empty() or not is_instance_valid(item_container):
		return

	var address := str(friend["address"]).to_lower()

	# Another card may have been created while awaiting
	if _current_addresses.has(address):
		return

	var item = FRIEND_DISCOVER_CARD.instantiate()
	item_container.add_child(item)
	item.set_data(place_data)
	item.item_pressed.connect(discover.on_friend_pressed)

	_current_addresses[address] = item

	_update_title()


func _update_card(address: String, friend: Dictionary) -> void:
	var card = _current_addresses.get(address)
	if not card or not is_instance_valid(card):
		return

	var place_data := await _build_place_data(friend)
	if place_data.is_empty() or not is_instance_valid(item_container):
		return

	card.set_data(place_data)


func _update_status() -> void:
	_update_title()
	if _current_addresses.is_empty():
		report_loading_status.emit(CarrouselGenerator.LoadingStatus.OK_WITHOUT_RESULTS)
	else:
		report_loading_status.emit(CarrouselGenerator.LoadingStatus.OK_WITH_RESULTS)


func _update_title() -> void:
	var carousel = get_parent()
	if not carousel or not "title" in carousel:
		return

	var count := _current_addresses.size()
	carousel.title = "Friends"

	# Create or update the count label next to the title
	var title_label = carousel.get_node_or_null("%Label_Title")
	if not title_label:
		return

	if _count_label == null:
		_count_label = Label.new()
		_count_label.add_theme_color_override("font_color", Color("E8B9FF"))
		_count_label.add_theme_font_override("font", title_label.get_theme_font("font"))
		_count_label.add_theme_font_size_override(
			"font_size", title_label.get_theme_font_size("font_size")
		)
		if title_label.label_settings:
			var ls = title_label.label_settings.duplicate()
			ls.font_color = Color("E8B9FF")
			_count_label.label_settings = ls
		var hbox = title_label.get_parent()
		hbox.add_theme_constant_override("separation", 10)
		hbox.add_child(_count_label)

	if count > 0:
		_count_label.text = "%d" % count
		_count_label.show()
	else:
		_count_label.hide()


func _remove_card(address: String) -> void:
	var card = _current_addresses.get(address)
	if card and is_instance_valid(card):
		item_container.remove_child(card)
		card.queue_free()
	_current_addresses.erase(address)


func _remove_all() -> void:
	for address in _current_addresses.keys():
		var card = _current_addresses[address]
		if is_instance_valid(card):
			item_container.remove_child(card)
			card.queue_free()
	_current_addresses.clear()


func clean_items():
	_remove_all()


# -- World fetching: streaming (first load) ------------------------------------


func _fetch_connected_worlds_streaming(addresses: Array, friends_by_address: Dictionary) -> void:
	if addresses.is_empty():
		return

	var base_url := DclUrls.worlds_content_server().replace("/world/", "")

	for address in addresses:
		var http_request = HTTPRequest.new()
		add_child(http_request)
		var url := base_url + "/wallet/" + str(address) + "/connected-world"
		http_request.request_completed.connect(
			_on_world_stream_completed.bind(address, http_request, friends_by_address)
		)
		var error = http_request.request(url)
		if error != OK:
			http_request.queue_free()


func _on_world_stream_completed(
	result: int,
	response_code: int,
	_headers: PackedStringArray,
	body: PackedByteArray,
	address: String,
	http_request: HTTPRequest,
	friends_by_address: Dictionary,
) -> void:
	http_request.queue_free()

	if not is_instance_valid(item_container):
		return
	if not friends_by_address.has(address):
		return
	if result != HTTPRequest.RESULT_SUCCESS or response_code != 200:
		return

	var json = JSON.new()
	if json.parse(body.get_string_from_utf8()) != OK:
		return

	var data = json.get_data()
	if not data is Dictionary:
		return

	var world_name: String = data.get("world", "")
	if world_name.is_empty():
		return

	if _current_addresses.has(address):
		return

	var friend = friends_by_address[address].duplicate()
	friend["world_name"] = world_name
	_async_create_friend_card(friend)


# -- World fetching: batch (refreshes) ----------------------------------------


func _async_fetch_connected_worlds(addresses: Array, friends_by_address: Dictionary) -> Dictionary:
	var result: Dictionary = {}
	if addresses.is_empty():
		return result

	var base_url := DclUrls.worlds_content_server().replace("/world/", "")
	var responses: Dictionary = {}  # address -> data Dictionary or null

	for address in addresses:
		var http_request = HTTPRequest.new()
		add_child(http_request)
		var url := base_url + "/wallet/" + str(address) + "/connected-world"
		http_request.request_completed.connect(
			_on_world_batch_completed.bind(address, http_request, responses)
		)
		var error = http_request.request(url)
		if error != OK:
			http_request.queue_free()
			responses[address] = null

	# Wait until all requests have responded
	while responses.size() < addresses.size():
		await get_tree().process_frame

	for address in responses.keys():
		var data = responses[address]
		if data == null or not data is Dictionary:
			continue
		var world_name: String = data.get("world", "")
		if world_name.is_empty():
			continue
		var friend = friends_by_address[address].duplicate()
		friend["world_name"] = world_name
		result[address] = friend

	return result


func _on_world_batch_completed(
	result: int,
	response_code: int,
	_headers: PackedStringArray,
	body: PackedByteArray,
	address: String,
	http_request: HTTPRequest,
	responses: Dictionary,
) -> void:
	http_request.queue_free()

	if result != HTTPRequest.RESULT_SUCCESS or response_code != 200:
		responses[address] = null
		return

	var json = JSON.new()
	if json.parse(body.get_string_from_utf8()) != OK:
		responses[address] = null
		return

	var data = json.get_data()
	if data is Dictionary:
		responses[address] = data
	else:
		responses[address] = null


func _async_await_with_timeout(promise_param: Promise, timeout_seconds: float) -> bool:
	if promise_param == null:
		return true
	if promise_param.is_resolved():
		return false

	var timer = get_tree().create_timer(timeout_seconds)
	while not promise_param.is_resolved() and timer.time_left > 0:
		await get_tree().process_frame

	return not promise_param.is_resolved()
