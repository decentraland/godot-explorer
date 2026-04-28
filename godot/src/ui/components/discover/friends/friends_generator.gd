extends CarrouselGenerator

const FRIEND_DISCOVER_CARD = preload(
	"res://src/ui/components/discover/friends/friend_discover_card.tscn"
)

const REFRESH_INTERVAL: float = 15.0

var _loading: bool = false
var _place_cache: Dictionary = {}  # "x,y" -> place_data
var _current_addresses: Dictionary = {}  # address_lower -> card node
var _connected_signals: bool = false
var _refresh_timer: Timer = null
var _first_load: bool = true


func _ready() -> void:
	_connect_realtime_signals()
	_refresh_timer = Timer.new()
	_refresh_timer.wait_time = REFRESH_INTERVAL
	_refresh_timer.timeout.connect(_on_refresh_timer)
	add_child(_refresh_timer)


func _connect_realtime_signals() -> void:
	if _connected_signals:
		return
	_connected_signals = true
	Global.social_service.friend_connectivity_updated.connect(_on_friend_connectivity_updated)


func _on_friend_connectivity_updated(_address: String, _status: int) -> void:
	if not _loading:
		on_request(0, 10)


func _on_refresh_timer() -> void:
	if not _loading and is_instance_valid(discover) and discover.is_visible_in_tree():
		on_request(0, 10)


func on_request(_offset: int, _limit: int) -> void:
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

	# Fetch peers from Archipelago to find friends in Genesis City
	Global.locations.fetch_peers()
	await Global.locations.in_genesis_city_changed

	var peers = Global.locations.in_genesis_city

	# Build desired set: address_lower -> friend data with parcel
	var desired: Dictionary = {}
	for peer in peers:
		var peer_address: String = str(peer["address"]).to_lower()
		if friends_by_address.has(peer_address):
			var friend = friends_by_address[peer_address].duplicate()
			friend["parcel"] = peer["parcel"]
			desired[peer_address] = friend

	# Remove cards for friends no longer in desired set
	var to_remove: Array = []
	for address in _current_addresses.keys():
		if not desired.has(address):
			to_remove.append(address)

	for address in to_remove:
		var card = _current_addresses[address]
		if is_instance_valid(card):
			item_container.remove_child(card)
			card.queue_free()
		_current_addresses.erase(address)

	# Add cards for new friends not yet displayed
	for address in desired.keys():
		if _current_addresses.has(address):
			continue
		var friend = desired[address]
		await _create_friend_card(friend)

	# Update visibility
	_first_load = false
	if _current_addresses.is_empty():
		report_loading_status.emit(CarrouselGenerator.LoadingStatus.OK_WITHOUT_RESULTS)
	else:
		report_loading_status.emit(CarrouselGenerator.LoadingStatus.OK_WITH_RESULTS)
		_refresh_timer.start()

	_loading = false


func _create_friend_card(friend: Dictionary) -> void:
	var parcel = friend["parcel"]
	var parcel_pos = Vector2i(int(parcel[0]), int(parcel[1]))
	var cache_key := "%d,%d" % [parcel_pos.x, parcel_pos.y]
	var place_name := ""
	var place_data: Dictionary = {}

	if _place_cache.has(cache_key):
		place_data = _place_cache[cache_key]
		place_name = place_data.get("title", "")
	else:
		var result = await PlacesHelper.async_get_by_position(parcel_pos)
		if result and not (result is PromiseError):
			var json: Dictionary = result.get_string_response_as_json()
			if not json.data.is_empty():
				place_data = json.data[0]
				place_name = place_data.get("title", "")
				_place_cache[cache_key] = place_data

	if place_name.is_empty():
		place_name = "%d, %d" % [parcel_pos.x, parcel_pos.y]

	if place_data.is_empty():
		place_data = {
			"title": place_name,
			"base_position": "%d,%d" % [parcel_pos.x, parcel_pos.y],
		}

	place_data["_friend_name"] = friend.get("name", friend["address"])
	place_data["_friend_address"] = friend["address"]
	place_data["_friend_profile_picture_url"] = friend.get("profile_picture_url", "")
	place_data["_friend_has_claimed_name"] = friend.get("has_claimed_name", false)

	var item = FRIEND_DISCOVER_CARD.instantiate()
	item_container.add_child(item)

	var label_title = item.get_node_or_null("%Label_Title")
	if label_title:
		label_title.text = friend.get("name", friend["address"])

	var label_location = item.get_node_or_null("%Label_Location")
	if label_location:
		label_location.text = place_name

	var profile_picture = item.get_node_or_null("%ProfilePicture")
	if profile_picture:
		var social_data = SocialItemData.new()
		social_data.name = friend.get("name", friend["address"])
		social_data.address = friend["address"]
		social_data.profile_picture_url = friend.get("profile_picture_url", "")
		social_data.has_claimed_name = friend.get("has_claimed_name", false)
		profile_picture.async_update_profile_picture(social_data)

	var checkmark = item.get_node_or_null("%TextureRect_ClaimedCheckmark")
	if checkmark:
		checkmark.visible = friend.get("has_claimed_name", false)

	item._data = place_data
	item.item_pressed.connect(discover.on_friend_pressed)

	_current_addresses[friend["address"].to_lower()] = item


func _remove_all() -> void:
	for address in _current_addresses.keys():
		var card = _current_addresses[address]
		if is_instance_valid(card):
			item_container.remove_child(card)
			card.queue_free()
	_current_addresses.clear()


func clean_items():
	_remove_all()


func _async_await_with_timeout(promise_param: Promise, timeout_seconds: float) -> bool:
	if promise_param == null:
		return true
	if promise_param.is_resolved():
		return false

	var timer = get_tree().create_timer(timeout_seconds)
	while not promise_param.is_resolved() and timer.time_left > 0:
		await get_tree().process_frame

	return not promise_param.is_resolved()
