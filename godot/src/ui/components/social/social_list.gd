class_name SocialList
extends Control

signal size_changed
signal load_error(error_message: String)

const SOCIAL_TYPE = SocialItemData.SocialType
const BLOCKED_AVATAR_ALIAS_BASE: int = 20000
const NEARBY_SYNC_INTERVAL: float = 5.0
const FRIENDS_ONLINE_OFFLINE_RECONCILE_INTERVAL: float = 15.0

@export var player_list_type: SocialItemData.SocialType

var list_size: int = 0
var has_error: bool = false
var _update_request_id: int = 0
var _nearby_sync_timer: Timer = null
var _friends_reconcile_timer: Timer = null
var _friends_reconcile_in_progress: bool = false


func _ready():
	# Don't auto-load on _ready() - lists will be loaded when show_panel() is called
	# This avoids race conditions with social service initialization
	if player_list_type == SOCIAL_TYPE.NEARBY:
		# Create sync timer for polling-based updates
		_nearby_sync_timer = Timer.new()
		_nearby_sync_timer.wait_time = NEARBY_SYNC_INTERVAL
		_nearby_sync_timer.timeout.connect(_sync_nearby_list)
		add_child(_nearby_sync_timer)
		_nearby_sync_timer.start()

		# Also sync on avatar signals for immediate response
		Global.avatars.avatar_added.connect(_on_avatar_changed)
		Global.avatars.avatar_removed.connect(_on_avatar_changed)

		# Update when blacklist changes to remove blocked users from nearby list
		Global.social_blacklist.blacklist_changed.connect(_async_on_blacklist_changed)
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

	# Periodic reconciliation for ONLINE/OFFLINE lists.
	# This corrects missed connectivity events by reconciling desired online/offline sets.
	if player_list_type == SOCIAL_TYPE.ONLINE or player_list_type == SOCIAL_TYPE.OFFLINE:
		_friends_reconcile_timer = Timer.new()
		_friends_reconcile_timer.wait_time = FRIENDS_ONLINE_OFFLINE_RECONCILE_INTERVAL
		_friends_reconcile_timer.timeout.connect(_async_on_friends_reconcile_timeout)
		add_child(_friends_reconcile_timer)
		_friends_reconcile_timer.start()


func _async_on_friends_reconcile_timeout() -> void:
	if not _is_panel_visible():
		return
	if _friends_reconcile_in_progress:
		return
	_friends_reconcile_in_progress = true
	# Run reconciliation using the existing ONLINE/OFFLINE reconcile logic.
	await async_update_list()
	_friends_reconcile_in_progress = false


func _on_avatar_changed(_arg = null) -> void:
	# Trigger sync when avatars change (for immediate response)
	if player_list_type != SOCIAL_TYPE.NEARBY:
		return
	_sync_nearby_list()


func _sync_nearby_list() -> void:
	if player_list_type != SOCIAL_TYPE.NEARBY:
		return

	# Get current avatar addresses (only those with valid data)
	var current_avatar_addresses: Dictionary = {}  # address -> Avatar
	var all_avatars = Global.avatars.get_avatars()

	for avatar in all_avatars:
		if avatar == null or not avatar is Avatar:
			continue
		# Skip avatars without valid address (not ready yet)
		if avatar.avatar_id.is_empty():
			continue
		# Skip blocked users (don't show skeletons/items for them)
		if Global.social_blacklist and Global.social_blacklist.is_blocked(avatar.avatar_id):
			continue
		current_avatar_addresses[avatar.avatar_id] = avatar

	# Get existing item addresses and check for removals
	var items_to_remove: Array = []
	var existing_addresses: Dictionary = {}  # address -> social_item

	for child in get_children():
		if not child is Control:
			continue
		if not "social_data" in child or child.social_data == null:
			# Item still loading without social_data, check if timed out
			if "load_state" in child and "is_load_timed_out" in child:
				if child.is_load_timed_out():
					child.mark_as_failed()
					items_to_remove.append(child)
				elif child.load_state == child.LoadState.FAILED:
					items_to_remove.append(child)
			continue

		var address = child.social_data.address

		# Check if should be removed:
		# 1. Address is empty
		# 2. Avatar no longer exists
		# 3. Item failed to load
		# 4. Item timed out while loading
		if address.is_empty():
			items_to_remove.append(child)
		elif not current_avatar_addresses.has(address):
			items_to_remove.append(child)
		elif child.load_state == child.LoadState.FAILED:
			items_to_remove.append(child)
		elif child.has_method("is_load_timed_out") and child.is_load_timed_out():
			child.mark_as_failed()
			items_to_remove.append(child)
		else:
			existing_addresses[address] = child

	# Remove items that should be removed
	for item in items_to_remove:
		item.queue_free()

	# Add items for new avatars
	for address in current_avatar_addresses:
		if not existing_addresses.has(address):
			var avatar = current_avatar_addresses[address]
			_add_item_for_avatar(avatar)

	# Update list size after changes
	# Use call_deferred to allow queue_free to complete
	call_deferred("_update_list_size")


func _add_item_for_avatar(avatar: Avatar) -> void:
	var social_item = Global.preload_assets.SOCIAL_ITEM.instantiate()
	add_child(social_item)
	social_item.visible = true
	social_item.set_type(player_list_type)
	social_item.set_data_from_avatar(avatar)


func _async_on_blacklist_changed() -> void:
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
		# Skip non-Control nodes (like Timer)
		if not child is Control:
			continue
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
	match player_list_type:
		SOCIAL_TYPE.NEARBY:
			# For NEARBY, use sync-based approach without clearing first.
			# Clearing causes a visible "blank gap" when reopening the panel: the tab can still show
			# a stale count while the list is empty until avatars are ready again.
			_sync_nearby_list()
			_update_list_size()
		SOCIAL_TYPE.BLOCKED:
			await _async_reload_blocked_list(current_request_id)
		SOCIAL_TYPE.ONLINE:
			await _async_reload_online_list(current_request_id)
		SOCIAL_TYPE.OFFLINE:
			await _async_reload_offline_list(current_request_id)
		SOCIAL_TYPE.REQUEST:
			remove_items()
			await _async_reload_request_list(current_request_id)


func _async_reload_blocked_list(request_id: int) -> void:
	var blocked_addresses = Global.social_blacklist.get_blocked_list()

	# Fetch all profiles in parallel
	var promises: Array = []
	for address in blocked_addresses:
		promises.append(Global.content_provider.fetch_profile(address))

	# Wait for all profile fetches to complete
	var blocked_social_items: Array = []
	for i in range(promises.size()):
		var address = blocked_addresses[i]
		await PromiseUtils.async_awaiter(promises[i])

		var social_item_data = SocialItemData.new()
		social_item_data.address = address

		if promises[i].is_rejected():
			social_item_data.name = address
			social_item_data.has_claimed_name = false
			social_item_data.profile_picture_url = ""
		else:
			var profile = promises[i].get_data() as DclUserProfile
			social_item_data.name = profile.get_name()
			social_item_data.has_claimed_name = profile.has_claimed_name()
			social_item_data.profile_picture_url = profile.get_avatar().get_snapshots_face_url()

		blocked_social_items.append(social_item_data)

	# Check if this request is still valid (no newer request started)
	if request_id != _update_request_id:
		return

	# Replace items only when all data is ready
	remove_items()
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
		remove_items()
		return

	has_error = false

	var friends_panel = _get_friends_panel()
	var offline_list = null
	if friends_panel != null:
		offline_list = friends_panel.offline_list

	# Desired ONLINE set: address_lower -> SocialItemData
	var desired_online := {}
	for friend in all_friends:
		if friends_panel and friends_panel.is_friend_online(friend.address):
			desired_online[friend.address.to_lower()] = friend

	var should_load := _is_panel_visible()

	# 1) Move any existing items that are no longer desired ONLINE to OFFLINE.
	# Copy children array because we'll move/remove during iteration.
	var children := get_children()
	for child in children:
		if not child is Control:
			continue
		if not ("social_data" in child and child.social_data != null):
			continue
		var key = child.social_data.address.to_lower()
		if not desired_online.has(key):
			# Move node to offline list if possible (no reload).
			if offline_list != null:
				remove_child(child)
				offline_list.add_child(child)
				child.set_type(SOCIAL_TYPE.OFFLINE)
			else:
				child.queue_free()

	# 2) Add missing ONLINE items (move from OFFLINE if already present).
	var existing := {}
	for child in get_children():
		if not child is Control:
			continue
		if not ("social_data" in child and child.social_data != null):
			continue
		existing[child.social_data.address.to_lower()] = true

	var to_add := []
	for key in desired_online.keys():
		if existing.has(key):
			continue
		var friend_data: SocialItemData = desired_online[key]
		var moved := false
		if offline_list != null and offline_list.has_item_with_address(friend_data.address):
			var node = offline_list.pop_item_by_address(friend_data.address)
			if node != null:
				add_child(node)
				node.set_type(SOCIAL_TYPE.ONLINE)
				moved = true
		if not moved:
			to_add.append(friend_data)

	if not to_add.is_empty():
		add_items_by_social_item_data(to_add, should_load)

	# Update sizes for both lists.
	_update_list_size()
	if offline_list != null:
		offline_list._update_list_size()


func _async_reload_offline_list(request_id: int) -> void:
	var all_friends = await _async_fetch_all_friends()

	# Check if this request is still valid (no newer request started)
	if request_id != _update_request_id:
		return

	if all_friends == null:
		# null means error occurred - don't show error for offline list, online list handles it
		list_size = 0
		size_changed.emit()
		remove_items()
		return

	var friends_panel = _get_friends_panel()
	var online_list = null
	if friends_panel != null:
		online_list = friends_panel.online_list

	# Desired OFFLINE set: address_lower -> SocialItemData
	var desired_offline := {}
	for friend in all_friends:
		if not friends_panel or not friends_panel.is_friend_online(friend.address):
			desired_offline[friend.address.to_lower()] = friend

	var should_load := _is_panel_visible()

	# 1) Move any existing items that are no longer desired OFFLINE to ONLINE.
	var children := get_children()
	for child in children:
		if not child is Control:
			continue
		if not ("social_data" in child and child.social_data != null):
			continue
		var key = child.social_data.address.to_lower()
		if not desired_offline.has(key):
			if online_list != null:
				remove_child(child)
				online_list.add_child(child)
				child.set_type(SOCIAL_TYPE.ONLINE)
			else:
				child.queue_free()

	# 2) Add missing OFFLINE items (move from ONLINE if already present).
	var existing := {}
	for child in get_children():
		if not child is Control:
			continue
		if not ("social_data" in child and child.social_data != null):
			continue
		existing[child.social_data.address.to_lower()] = true

	var to_add := []
	for key in desired_offline.keys():
		if existing.has(key):
			continue
		var friend_data: SocialItemData = desired_offline[key]
		var moved := false
		if online_list != null and online_list.has_item_with_address(friend_data.address):
			var node = online_list.pop_item_by_address(friend_data.address)
			if node != null:
				add_child(node)
				node.set_type(SOCIAL_TYPE.OFFLINE)
				moved = true
		if not moved:
			to_add.append(friend_data)

	if not to_add.is_empty():
		add_items_by_social_item_data(to_add, should_load)

	_update_list_size()
	if online_list != null:
		online_list._update_list_size()


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
		item.friendship_id = req.get("friendship_id", "")
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


func remove_items() -> void:
	for child in self.get_children():
		# Don't remove the sync timer
		if child == _nearby_sync_timer:
			continue
		child.queue_free()


func remove_item_by_address(address: String) -> bool:
	# Remove a specific item by address without reloading the list
	# Returns true if item was found and removed
	var address_lower = address.to_lower()
	for child in get_children():
		if "social_data" in child and child.social_data != null:
			if child.social_data.address.to_lower() == address_lower:
				child.queue_free()
				# Update list size immediately (decrement since queue_free is deferred)
				list_size = maxi(0, list_size - 1)
				size_changed.emit()
				return true
	return false


func pop_item_by_address(address: String) -> Control:
	# Remove and return a specific SocialItem node by address without freeing it.
	# Used to move items between ONLINE/OFFLINE lists without triggering skeleton/load.
	for child in get_children():
		if not child is Control:
			continue
		if "social_data" in child and child.social_data != null:
			if child.social_data.address.to_lower() == address.to_lower():
				remove_child(child)
				return child
	return null


func has_item_with_address(address: String) -> bool:
	# Check if an item with the given address exists in the list
	var address_lower = address.to_lower()
	for child in get_children():
		if "social_data" in child and child.social_data != null:
			if child.social_data.address.to_lower() == address_lower:
				return true
	return false


func get_item_data_by_address(address: String) -> SocialItemData:
	# Get the SocialItemData for an item by address (returns null if not found)
	var address_lower = address.to_lower()
	for child in get_children():
		if "social_data" in child and child.social_data != null:
			if child.social_data.address.to_lower() == address_lower:
				return child.social_data
	return null


func add_item_by_social_item_data(item: SocialItemData, should_load: bool = true) -> void:
	# Add a single item to the list without clearing existing items
	var social_item = Global.preload_assets.SOCIAL_ITEM.instantiate()
	self.add_child(social_item)
	social_item.visible = true
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


func sync_items(new_items: Array) -> void:
	# Build dict of existing items by address
	var existing: Dictionary = {}
	for child in get_children():
		if child == _nearby_sync_timer:
			continue
		if "social_data" in child and child.social_data != null:
			existing[child.social_data.address] = child

	# Build dict of incoming items by address
	var incoming: Dictionary = {}
	for item in new_items:
		incoming[item.address] = item

	# Remove items no longer in the list
	for address in existing:
		if not incoming.has(address):
			existing[address].queue_free()

	# Add items that are new
	var should_load = _is_panel_visible()
	for address in incoming:
		if not existing.has(address):
			add_item_by_social_item_data(incoming[address], should_load)

	call_deferred("_update_list_size")


func add_items_by_social_item_data(item_list, should_load: bool = true) -> void:
	for item in item_list:
		var social_item = Global.preload_assets.SOCIAL_ITEM.instantiate()
		self.add_child(social_item)
		social_item.visible = true
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
