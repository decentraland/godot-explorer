class_name SocialList
extends Control

signal size_changed
signal load_error(error_message: String)

const SOCIAL_TYPE = SocialItemData.SocialType
const BLOCKED_AVATAR_ALIAS_BASE: int = 20000

@export var player_list_type: SocialItemData.SocialType

var list_size: int = 0
var has_error: bool = false
var _update_request_id: int = 0


func _ready():
	# Don't auto-load on _ready() - lists will be loaded when show_panel() is called
	# This avoids race conditions with social service initialization
	if player_list_type == SOCIAL_TYPE.NEARBY:
		# Use individual avatar signals instead of refreshing the whole list
		Global.avatars.avatar_added.connect(_on_avatar_added)
		Global.avatars.avatar_removed.connect(_on_avatar_removed)
		# Also update when blacklist changes to remove blocked users from nearby list
		Global.social_blacklist.blacklist_changed.connect(_on_blacklist_changed)
		# Update nearby items when friendship requests change
		Global.social_service.friendship_request_received.connect(_on_friendship_request_received)
		Global.social_service.friendship_request_accepted.connect(_on_friendship_request_changed)
		Global.social_service.friendship_request_rejected.connect(_on_friendship_request_changed)
		Global.social_service.friendship_request_cancelled.connect(_on_friendship_request_changed)
		Global.social_service.friendship_deleted.connect(_on_friendship_request_changed)
	if player_list_type == SOCIAL_TYPE.BLOCKED:
		Global.social_blacklist.blacklist_changed.connect(self.async_update_list)
	if player_list_type == SOCIAL_TYPE.REQUEST:
		# Reload request list when blacklist changes to pick up previously hidden requests
		Global.social_blacklist.blacklist_changed.connect(self.async_update_list)


func _on_avatar_added(avatar: Avatar) -> void:
	if player_list_type != SOCIAL_TYPE.NEARBY:
		return

	# Check if item with this address already exists to prevent duplicates
	if not avatar.avatar_id.is_empty():
		if has_item_with_address(avatar.avatar_id):
			return

	# Create a new social item for this avatar
	# The item will handle its own visibility based on blocked status
	var social_item = Global.preload_assets.SOCIAL_ITEM.instantiate()
	self.add_child(social_item)
	social_item.set_type(player_list_type)
	social_item.set_data_from_avatar(avatar)


func _on_avatar_removed(address: String) -> void:
	if player_list_type != SOCIAL_TYPE.NEARBY:
		return

	# Find and remove the social item for this address
	for child in get_children():
		if child.has_method("get") and child.get("social_data") != null:
			var social_data = child.social_data
			if social_data != null and social_data.address == address:
				child.queue_free()
				_update_list_size()
				return


func _on_blacklist_changed() -> void:
	# Handle blacklist changes for NEARBY and REQUEST lists
	if player_list_type != SOCIAL_TYPE.NEARBY and player_list_type != SOCIAL_TYPE.REQUEST:
		return

	# Items will handle their own visibility via blacklist_changed signal
	# Wait a frame for items to update their visibility, then update list size
	await get_tree().process_frame
	_update_list_size()


func _on_friendship_request_received(_unused_address: String, _unused_message: String) -> void:
	_update_nearby_friend_status()


func _on_friendship_request_changed(_unused_address: String) -> void:
	_update_nearby_friend_status()


func _update_nearby_friend_status() -> void:
	if player_list_type != SOCIAL_TYPE.NEARBY:
		return
	# Update friendship status on existing items instead of reloading
	for child in get_children():
		if child.has_method("_check_and_update_friend_status"):
			child._check_and_update_friend_status()


func _update_list_size() -> void:
	var visible_count = 0
	for child in get_children():
		if child.visible:
			visible_count += 1
	list_size = visible_count
	size_changed.emit()


func _request_reorder() -> void:
	# Called by social_item when friendship status is determined
	# Debounce reordering to avoid multiple reorders in quick succession
	if player_list_type != SOCIAL_TYPE.NEARBY:
		return

	# Use call_deferred to batch multiple reorder requests
	call_deferred("_reorder_items")


func _reorder_items() -> void:
	# Use insertion sort to maintain order: friends first, then non-friends, alphabetically
	# This is more efficient than rebuilding the entire order for small changes
	var children = get_children()
	if children.size() <= 1:
		return

	# Insertion sort: for each item, find its correct position and move it there
	for i in range(1, children.size()):
		var current = children[i]
		if not is_instance_valid(current):
			continue

		var j = i - 1
		# Find the correct position for current item
		while j >= 0 and _should_come_before(current, children[j]):
			j -= 1

		# Move current to position j + 1 if different from current position
		var target_pos = j + 1
		if target_pos != i:
			move_child(current, target_pos)
			# Update children array to reflect the move
			children = get_children()


func _should_come_before(a, b) -> bool:
	# Returns true if 'a' should come before 'b' in the sorted order
	# Order: friends first (alphabetically), then non-friends (alphabetically)
	if not is_instance_valid(a) or not is_instance_valid(b):
		return false

	var a_is_friend = a.has_method("is_friend") and a.is_friend()
	var b_is_friend = b.has_method("is_friend") and b.is_friend()

	# Friends come before non-friends
	if a_is_friend and not b_is_friend:
		return true
	if not a_is_friend and b_is_friend:
		return false

	# Same category: sort alphabetically by name
	return _compare_by_name(a, b)


func _compare_by_name(a, b) -> bool:
	var name_a = ""
	var name_b = ""
	if "social_data" in a and a.social_data != null:
		name_a = a.social_data.name.to_lower()
	if "social_data" in b and b.social_data != null:
		name_b = b.social_data.name.to_lower()
	return name_a < name_b


func async_update_list(_remote_avatars: Array = []) -> void:
	# Increment request ID to invalidate any in-flight requests
	_update_request_id += 1
	var current_request_id = _update_request_id

	remove_items()
	match player_list_type:
		SOCIAL_TYPE.NEARBY:
			# For NEARBY, load existing avatars on initial call
			# After that, new avatars are added via avatar_added signal
			_load_existing_nearby_avatars()
		SOCIAL_TYPE.BLOCKED:
			await _async_reload_blocked_list(current_request_id)
		SOCIAL_TYPE.ONLINE:
			await _async_reload_online_list(current_request_id)
		SOCIAL_TYPE.OFFLINE:
			await _async_reload_offline_list(current_request_id)
		SOCIAL_TYPE.REQUEST:
			await _async_reload_request_list(current_request_id)


func _async_reload_blocked_list(request_id: int) -> void:
	var blocked_social_items = []
	var blocked_addresses = Global.social_blacklist.get_blocked_list()

	# Fetch profiles for each blocked address
	for address in blocked_addresses:
		# Check if request is still valid before each fetch
		if request_id != _update_request_id:
			return

		var social_item_data = await _async_fetch_profile_for_address(address)
		blocked_social_items.append(social_item_data)

	# Check if this request is still valid (no newer request started)
	if request_id != _update_request_id:
		return

	var should_load = _is_panel_visible()
	add_items_by_social_item_data(blocked_social_items, should_load)
	_update_list_size()


func _async_reload_online_list(request_id: int) -> void:
	var all_friends = await _async_fetch_all_friends()

	# Check if this request is still valid (no newer request started)
	if request_id != _update_request_id:
		return

	if all_friends == null:
		# null means error occurred
		has_error = true
		list_size = 0
		load_error.emit("Friends service unavailable")
		size_changed.emit()
		return

	has_error = false

	# Filter to only show friends that are ONLINE
	var online_friends = []
	var friends_panel = _get_friends_panel()
	for friend in all_friends:
		var is_online = friends_panel and friends_panel.is_friend_online(friend.address)
		if is_online:
			online_friends.append(friend)

	if online_friends.is_empty():
		list_size = 0
		size_changed.emit()
		return

	var should_load = _is_panel_visible()
	add_items_by_social_item_data(online_friends, should_load)
	_update_list_size()


func _async_reload_offline_list(request_id: int) -> void:
	var all_friends = await _async_fetch_all_friends()

	# Check if this request is still valid (no newer request started)
	if request_id != _update_request_id:
		return

	if all_friends == null:
		# null means error occurred - don't show error for offline list, online list handles it
		list_size = 0
		size_changed.emit()
		return

	# Filter to only show friends that are OFFLINE (not in online tracking)
	var offline_friends = []
	var friends_panel = _get_friends_panel()
	for friend in all_friends:
		if not friends_panel or not friends_panel.is_friend_online(friend.address):
			offline_friends.append(friend)

	if offline_friends.is_empty():
		list_size = 0
		size_changed.emit()
		return

	var should_load = _is_panel_visible()
	add_items_by_social_item_data(offline_friends, should_load)
	_update_list_size()


func _get_friends_panel():
	# Navigate up the tree to find the friends panel
	var parent = get_parent()
	while parent != null:
		if parent.has_method("is_friend_online"):
			return parent
		parent = parent.get_parent()
	return null


func _async_reload_request_list(request_id: int) -> void:
	var promise = Global.social_service.get_pending_requests(100, 0)
	await PromiseUtils.async_awaiter(promise)

	# Check if this request is still valid (no newer request started)
	if request_id != _update_request_id:
		return

	if promise.is_rejected():
		printerr("Failed to load pending requests: ", PromiseUtils.get_error_message(promise))
		has_error = true
		list_size = 0
		load_error.emit("Friends service unavailable")
		size_changed.emit()
		return

	has_error = false
	var requests = promise.get_data()
	var request_items = []

	for req in requests:
		var item = SocialItemData.new()
		item.address = req["address"]
		item.name = req["name"]
		item.has_claimed_name = req["has_claimed_name"]
		item.profile_picture_url = req["profile_picture_url"]
		request_items.append(item)

	var should_load = _is_panel_visible()
	add_items_by_social_item_data(request_items, should_load)
	# Wait a frame for items to check their blocked status and update visibility
	await get_tree().process_frame
	_update_list_size()


func _async_fetch_all_friends():
	# Fetch all friends (status=-1 for all)
	# Returns null on error, empty array if no friends, array of items otherwise
	var promise = Global.social_service.get_friends(100, 0, -1)

	# Use timeout to prevent hanging forever (10 seconds)
	var timed_out = await _async_await_with_timeout(promise, 10.0)
	if timed_out:
		return null

	if promise.is_rejected():
		return null

	var friends = promise.get_data()
	var friend_items = []

	for friend_data in friends:
		var item = SocialItemData.new()
		item.address = friend_data["address"]
		item.name = friend_data["name"]
		item.has_claimed_name = friend_data["has_claimed_name"]
		item.profile_picture_url = friend_data["profile_picture_url"]
		friend_items.append(item)

	return friend_items


func _async_await_with_timeout(promise: Promise, timeout_seconds: float) -> bool:
	# Returns true if timed out, false if promise resolved
	if promise == null:
		return true
	if promise.is_resolved():
		return false

	var timer = get_tree().create_timer(timeout_seconds)
	var resolved = false

	# Wait for either promise resolution or timeout
	while not resolved and timer.time_left > 0:
		if promise.is_resolved():
			resolved = true
			break
		await get_tree().process_frame

	return not resolved


func _load_existing_nearby_avatars() -> void:
	# Load any avatars that already exist when the list is first shown
	var all_avatars = Global.avatars.get_avatars()

	for avatar in all_avatars:
		if avatar != null and avatar is Avatar:
			# Skip if item with this address already exists to prevent duplicates
			if not avatar.avatar_id.is_empty():
				if has_item_with_address(avatar.avatar_id):
					continue

			# Create item - it will handle its own loading and visibility
			# Items will hide themselves if blocked via _update_blocked_visibility()
			var social_item = Global.preload_assets.SOCIAL_ITEM.instantiate()
			self.add_child(social_item)
			social_item.set_type(player_list_type)
			social_item.set_data_from_avatar(avatar)

	_update_list_size()


func remove_items() -> void:
	for child in self.get_children():
		child.queue_free()


func remove_item_by_address(address: String) -> bool:
	# Remove a specific item by address without reloading the list
	# Returns true if item was found and removed
	for child in get_children():
		if "social_data" in child and child.social_data != null:
			if child.social_data.address == address:
				child.queue_free()
				# Update list size immediately (decrement since queue_free is deferred)
				list_size = maxi(0, list_size - 1)
				size_changed.emit()
				return true
	return false


func has_item_with_address(address: String) -> bool:
	# Check if an item with the given address exists in the list
	for child in get_children():
		if "social_data" in child and child.social_data != null:
			if child.social_data.address == address:
				return true
	return false


func get_item_data_by_address(address: String) -> SocialItemData:
	# Get the SocialItemData for an item by address (returns null if not found)
	for child in get_children():
		if "social_data" in child and child.social_data != null:
			if child.social_data.address == address:
				return child.social_data
	return null


func add_item_by_social_item_data(item: SocialItemData, should_load: bool = true) -> void:
	# Add a single item to the list without clearing existing items
	var social_item = Global.preload_assets.SOCIAL_ITEM.instantiate()
	self.add_child(social_item)
	social_item.set_type(player_list_type)
	social_item.set_data(item, should_load)
	_update_list_size()


func async_add_request_by_address(address: String) -> void:
	# Add a friend request item by fetching the profile (for REQUEST list type)
	# Skip if item already exists
	if has_item_with_address(address):
		return

	var social_item_data = await _async_fetch_profile_for_address(address)
	# Double-check it wasn't added while we were fetching
	if not has_item_with_address(address):
		var should_load = _is_panel_visible()
		add_item_by_social_item_data(social_item_data, should_load)


func add_items_by_social_item_data(item_list, should_load: bool = true) -> void:
	for item in item_list:
		var social_item = Global.preload_assets.SOCIAL_ITEM.instantiate()
		self.add_child(social_item)
		social_item.set_type(player_list_type)
		social_item.set_data(item as SocialItemData, should_load)


func load_unloaded_items() -> void:
	for child in get_children():
		if child.has_method("load_item") and "load_state" in child:
			if child.load_state == child.LoadState.UNLOADED:
				child.load_item()


func _is_panel_visible() -> bool:
	var friends_panel = _get_friends_panel()
	return friends_panel != null and friends_panel.visible


func _async_fetch_profile_for_address(address: String) -> SocialItemData:
	var social_item_data = SocialItemData.new()
	social_item_data.address = address

	var promise = Global.content_provider.fetch_profile(address)
	var result = await PromiseUtils.async_awaiter(promise)

	if result is PromiseError:
		# Fallback to address if profile fetch fails
		social_item_data.name = address
		social_item_data.has_claimed_name = false
		social_item_data.profile_picture_url = ""
	else:
		var profile = result as DclUserProfile
		social_item_data.name = profile.get_name()
		social_item_data.has_claimed_name = profile.has_claimed_name()
		social_item_data.profile_picture_url = profile.get_avatar().get_snapshots_face_url()

	return social_item_data
