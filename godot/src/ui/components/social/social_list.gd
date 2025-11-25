class_name SocialList
extends Control

signal size_changed
signal load_error(error_message: String)

enum SocialType { ONLINE, OFFLINE, REQUEST, NEARBY, BLOCKED }

const BLOCKED_AVATAR_ALIAS_BASE: int = 20000

@export var player_list_type: SocialType

var list_size: int = 0
var has_error: bool = false
var _update_request_id: int = 0


func _ready():
	async_update_list()
	if player_list_type == SocialType.NEARBY:
		Global.avatars.avatar_scene_changed.connect(self.async_update_list)
	if player_list_type == SocialType.BLOCKED:
		Global.social_blacklist.blacklist_changed.connect(self.async_update_list)
	#Global.get_explorer().hud_button_friends.friends_clicked.connect(self.async_update_list)


func async_update_list(_remote_avatars: Array = []) -> void:
	# Increment request ID to invalidate any in-flight requests
	_update_request_id += 1
	var current_request_id = _update_request_id

	remove_items()
	match player_list_type:
		SocialType.NEARBY:
			_reload_nearby_list()
		SocialType.BLOCKED:
			await _async_reload_blocked_list(current_request_id)
		SocialType.ONLINE:
			await _async_reload_online_list(current_request_id)
		SocialType.OFFLINE:
			await _async_reload_offline_list(current_request_id)
		SocialType.REQUEST:
			await _async_reload_request_list(current_request_id)
		_:
			pass


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

	add_items_by_social_item_data(blocked_social_items)
	list_size = blocked_social_items.size()
	size_changed.emit()


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
		if friends_panel and friends_panel.is_friend_online(friend.address):
			online_friends.append(friend)

	if online_friends.is_empty():
		list_size = 0
		size_changed.emit()
		return

	add_items_by_social_item_data(online_friends)
	list_size = online_friends.size()
	size_changed.emit()


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

	add_items_by_social_item_data(offline_friends)
	list_size = offline_friends.size()
	size_changed.emit()


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
		printerr("Failed to load pending requests: ", promise.get_data().get_error())
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

	add_items_by_social_item_data(request_items)
	list_size = request_items.size()
	size_changed.emit()


func _async_fetch_all_friends():
	# Fetch all friends (status=-1 for all)
	# Returns null on error, empty array if no friends, array of items otherwise
	var promise = Global.social_service.get_friends(100, 0, -1)
	await PromiseUtils.async_awaiter(promise)

	if promise.is_rejected():
		printerr("Failed to load friends: ", promise.get_data().get_error())
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


func _reload_nearby_list() -> void:
	var all_avatars = Global.avatars.get_avatars()
	var avatars = []
	var seen_addresses = {}  # Diccionario para rastrear direcciones ya agregadas

	for avatar in all_avatars:
		if avatar != null and avatar is Avatar:
			var avatar_address = avatar.avatar_id
			if (
				not avatar_address.is_empty()
				and not Global.social_blacklist.is_blocked(avatar_address)
			):
				# Verificar si ya agregamos este avatar_id para evitar duplicados
				if not seen_addresses.has(avatar_address):
					seen_addresses[avatar_address] = true
					avatars.append(avatar)

	list_size = avatars.size()
	size_changed.emit()
	add_items_by_avatar(avatars)


func _compare_names(a, b):
	return a.social_data.name < b.social_data.name


func remove_items() -> void:
	for child in self.get_children():
		child.queue_free()


func add_items_by_avatar(avatar_list) -> void:
	for avatar in avatar_list:
		var social_item = Global.preload_assets.SOCIAL_ITEM.instantiate()
		self.add_child(social_item)
		social_item.set_type(player_list_type)
		social_item.set_data_from_avatar(avatar as Avatar)


func add_items_by_social_item_data(item_list) -> void:
	for item in item_list:
		var social_item = Global.preload_assets.SOCIAL_ITEM.instantiate()
		self.add_child(social_item)
		social_item.set_type(player_list_type)
		social_item.set_data(item as SocialItemData)

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

	
