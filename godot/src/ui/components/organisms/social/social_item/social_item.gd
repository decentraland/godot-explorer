extends Control

enum LoadState { UNLOADED, LOADING, LOADED, FAILED }

const SOCIAL_TYPE = SocialItemData.SocialType
const LOAD_TIMEOUT_SECONDS: float = 5.0
const MIN_SKELETON_VISIBLE_SECONDS: float = 0.3

@export var item_type: SocialItemData.SocialType

var is_guest = false
var social_data: SocialItemData
var current_friendship_status: int = Global.FriendshipStatus.UNKNOWN
var load_state: LoadState = LoadState.UNLOADED
var parcel: Array = []  # Parcel coordinates [x, y] when user is in genesis city
# ENS name of the world the friend is in (empty when in Genesis City or nowhere). Read from
# Global.locations.online_locations; drives the world place label and jump-in.
var world_name: String = ""
var _avatar_ref: WeakRef = null  # Weak reference to avatar for nearby items
var _is_loading: bool = false
var _load_start_time: float = 0.0

@onready var h_box_container_online: HBoxContainer = %HBoxContainer_Online
@onready var h_box_container_nearby: HBoxContainer = %HBoxContainer_Nearby
@onready var h_box_container_request: HBoxContainer = %HBoxContainer_Request
@onready var h_box_container_blocked: HBoxContainer = %HBoxContainer_Blocked
@onready var nickname: Label = %Nickname
@onready var label_place: Label = %Label_Place
@onready var profile_picture: ProfilePicture = %ProfilePicture
@onready var v_box_container_nickname: VBoxContainer = %VBoxContainer_Nickname
@onready var texture_rect_claimed_checkmark: TextureRect = %TextureRect_ClaimedCheckmark
@onready var button_add_friend: Button = %Button_AddFriend
@onready var button_accept: Button = %Button_Accept
@onready var button_reject: Button = %Button_Reject
@onready var button_jump: Button = %Button_JumpIn
@onready var button_profile: Button = %Button_Profile
@onready var data_container: HBoxContainer = %Data
@onready var skeleton_container: HBoxContainer = %Skeleton
@onready var panel_container_request: PanelContainer = %PanelContainer_Request
@onready var button_unblock: Button = %Button_Unblock


func _ready():
	add_to_group("blacklist_ui_sync")
	_set_loading(true)
	_update_elements_visibility()
	# Connect accept/reject buttons for friend requests
	button_accept.pressed.connect(_async_on_button_accept_pressed)
	button_add_friend.pressed.connect(_on_button_add_friend_pressed)
	button_reject.pressed.connect(_async_on_button_reject_pressed)
	button_jump.pressed.connect(_on_button_jump_in_pressed)
	button_unblock.pressed.connect(_on_button_unblock_pressed)
	# Profile opens from Button_Profile (its own drag-vs-tap detection), not the whole row.
	button_profile.pressed.connect(_on_pressed)
	# Connect to locations signal to update jump button visibility
	if Global.locations:
		Global.locations.online_locations_changed.connect(_on_online_locations_changed)

	# Place label ellipsizes on width (clip_text lets the overrun trim to the available room)
	# instead of a fixed character cap that cut long titles short while space was left.
	label_place.clip_text = true


func set_data(data: SocialItemData, should_load: bool = true) -> void:
	social_data = data
	_apply_data_to_ui()

	if should_load:
		load_item()
	else:
		load_state = LoadState.UNLOADED

	# Update jump button visibility if type is ONLINE
	if item_type == SOCIAL_TYPE.ONLINE:
		_update_jump_button_visibility()

	# Check blocked visibility for NEARBY and REQUEST items
	_update_blocked_visibility_for_type()


func _apply_data_to_ui() -> void:
	if social_data == null:
		return

	var display_name = social_data.name
	var tag_position = display_name.find("#")
	if tag_position != -1:
		display_name = display_name.left(tag_position)
		texture_rect_claimed_checkmark.hide()
	else:
		texture_rect_claimed_checkmark.show()

	# The nickname row (EllipsisNameLabel) clips on width now, so keep the full name here instead
	# of a hard character cap.
	nickname.text = display_name

	var nickname_color = DclAvatar.get_nickname_color(social_data.name)
	nickname.add_theme_color_override("font_color", nickname_color)
	if social_data.has_claimed_name:
		texture_rect_claimed_checkmark.show()
	else:
		texture_rect_claimed_checkmark.hide()

	# Name text and check visibility both changed; re-fit so the check stays glued to the name.
	var nickname_row := nickname.get_parent()
	if nickname_row is EllipsisNameLabel:
		nickname_row.refresh.call_deferred()


func load_item() -> void:
	if load_state == LoadState.LOADED or load_state == LoadState.LOADING:
		return
	if social_data == null:
		return

	load_state = LoadState.LOADING
	_load_start_time = Time.get_unix_time_from_system()
	_set_loading(true)
	_async_load_item()


func is_load_timed_out() -> bool:
	# Timeout while either:
	# 1) loading the profile picture (load_state == LOADING)
	# 2) waiting for the avatar to become ready (avatar_ready not yet set).
	# In the second case, load_state can still be UNLOADED while `_is_loading` is true.
	if load_state == LoadState.LOADED or load_state == LoadState.FAILED:
		return false

	# Waiting for avatar readiness
	if load_state == LoadState.UNLOADED and _is_loading:
		return Time.get_unix_time_from_system() - _load_start_time > LOAD_TIMEOUT_SECONDS

	# Loading profile picture
	if load_state == LoadState.LOADING:
		return Time.get_unix_time_from_system() - _load_start_time > LOAD_TIMEOUT_SECONDS

	return false


func mark_as_failed() -> void:
	load_state = LoadState.FAILED
	_is_loading = false
	_set_loading(false)


func _async_load_item() -> void:
	await profile_picture.async_update_profile_picture(social_data)
	if load_state == LoadState.FAILED:
		_set_loading(false)
		return

	# If the profile picture loads from cache, the skeleton can disappear too quickly
	# to be perceived. Keep it visible for a minimum duration for UX consistency.
	var elapsed := Time.get_unix_time_from_system() - _load_start_time
	if elapsed < MIN_SKELETON_VISIBLE_SECONDS:
		await get_tree().create_timer(MIN_SKELETON_VISIBLE_SECONDS - elapsed).timeout

	# If type is NEARBY, check if already a friend (async) while still loading
	if item_type == SOCIAL_TYPE.NEARBY and not social_data.address.is_empty():
		await _async_check_friend_status()

	load_state = LoadState.LOADED
	_set_loading(false)


func set_data_from_avatar(avatar_param: Avatar) -> void:
	_avatar_ref = weakref(avatar_param)

	# Show self with skeleton while loading
	_is_loading = true
	visible = true
	_load_start_time = Time.get_unix_time_from_system()
	_set_loading(true)

	# If avatar is not ready, wait for it
	if not avatar_param.avatar_ready:
		avatar_param.avatar_loaded.connect(_on_avatar_loaded, CONNECT_ONE_SHOT)
		return

	# Avatar is ready, load data immediately
	_load_data_from_avatar(avatar_param)


func _on_avatar_loaded() -> void:
	var avatar = _avatar_ref.get_ref() as Avatar if _avatar_ref else null
	if avatar == null or not is_instance_valid(avatar):
		# Avatar was freed, mark as failed
		mark_as_failed()
		return

	_load_data_from_avatar(avatar)


func _load_data_from_avatar(avatar_param: Avatar) -> void:
	# Check if avatar_id is set (it should be after avatar_ready)
	if avatar_param.avatar_id.is_empty():
		# Still no avatar_id, mark as failed
		mark_as_failed()
		return

	# Check for duplicates - another item with same address may already exist
	var parent_list = get_parent()
	if parent_list and parent_list.has_method("has_item_with_address"):
		if parent_list.has_item_with_address(avatar_param.avatar_id):
			# Duplicate found, mark as failed (will be cleaned up by sync)
			mark_as_failed()
			return

	# Check if avatar has valid data
	var avatar_data = avatar_param.get_avatar_data()
	if avatar_data == null:
		mark_as_failed()
		return

	social_data = SocialItemData.new()
	social_data.name = avatar_param.get_avatar_name()
	social_data.address = avatar_param.avatar_id
	social_data.profile_picture_url = avatar_data.get_snapshots_face_url()

	# Validate we got a name (profile might have failed to load)
	if social_data.name.is_empty():
		mark_as_failed()
		return

	social_data.has_claimed_name = false if social_data.name.contains("#") else true

	# Check if user is a guest (hasn't connected web3 wallet)
	is_guest = not avatar_param.has_connected_web3

	# Show self and set data
	_is_loading = false
	set_data(social_data)

	# Notify parent that we're ready (for list size updates)
	_notify_parent_size_changed()


func _notify_other_components_of_change() -> void:
	if social_data.address:
		Global.get_tree().call_group("blacklist_ui_sync", "_sync_blacklist_ui", social_data.address)


func _sync_blacklist_ui(changed_avatar_id: String) -> void:
	if social_data and social_data.address == changed_avatar_id:
		call_deferred("_update_blocked_visibility_for_type")


func _update_elements_visibility() -> void:
	_hide_all_buttons()
	match item_type:
		SOCIAL_TYPE.NEARBY:
			h_box_container_nearby.show()
			if Global.player_identity.is_guest or is_guest:
				button_add_friend.hide()
				return
			# Guest users cannot add friends
			# Check if already a friend and hide/show ADD FRIEND button accordingly
			if social_data and not social_data.address.is_empty():
				# If status is already known (pre-checked), use it directly
				if current_friendship_status != Global.FriendshipStatus.UNKNOWN:
					_update_button_visibility_from_status()
				else:
					# Hide button initially to avoid flickering, will show/hide after checking status
					button_add_friend.hide()
					_check_and_update_friend_status()
		SOCIAL_TYPE.ONLINE:
			h_box_container_online.show()
			_update_jump_button_visibility()
		SOCIAL_TYPE.REQUEST:
			h_box_container_request.show()
			# Guest users cannot accept/reject friend requests
			if Global.player_identity.is_guest or is_guest:
				button_accept.hide()
				button_reject.hide()
		SOCIAL_TYPE.REQUEST_SENT:
			# Sent requests can only be cancelled: show just the reject (X) button as "cancel".
			h_box_container_request.show()
			button_accept.hide()
			if Global.player_identity.is_guest or is_guest:
				button_reject.hide()
		SOCIAL_TYPE.BLOCKED:
			h_box_container_blocked.show()
		_:
			pass


func _hide_all_buttons() -> void:
	h_box_container_online.hide()
	h_box_container_nearby.hide()
	h_box_container_request.hide()
	h_box_container_blocked.hide()
	label_place.hide()
	button_add_friend.hide()
	panel_container_request.hide()


func _notify_parent_size_changed() -> void:
	var parent = get_parent()
	if parent and parent.has_method("_update_list_size"):
		parent._update_list_size()


func set_type(type: SocialItemData.SocialType) -> void:
	item_type = type
	_update_elements_visibility()
	# Subscribe to blacklist changes for NEARBY and REQUEST items to hide/show themselves
	if (
		(
			item_type == SOCIAL_TYPE.NEARBY
			or item_type == SOCIAL_TYPE.REQUEST
			or item_type == SOCIAL_TYPE.REQUEST_SENT
		)
		and not Global.social_blacklist.blacklist_changed.is_connected(_on_blacklist_changed)
	):
		Global.social_blacklist.blacklist_changed.connect(_on_blacklist_changed)
	# Update visibility based on blocked status after setting type
	_update_blocked_visibility_for_type()


func _on_button_add_friend_pressed() -> void:
	# ADD FRIEND button only sends friend requests (original behavior)
	_async_on_button_add_friend_pressed()


func _async_on_button_add_friend_pressed() -> void:
	button_add_friend.disabled = true
	var promise = Global.social_service.send_friend_request(social_data.address, "")
	await PromiseUtils.async_awaiter(promise)

	if promise.is_rejected():
		printerr("Failed to send friend request: ", PromiseUtils.get_error_message(promise))
		button_add_friend.disabled = false
		return

	# Request Friend metric
	Global.metrics.track_request_friend(social_data.address)

	current_friendship_status = Global.FriendshipStatus.REQUEST_SENT
	button_add_friend.hide()
	panel_container_request.show()
	# Now pending: re-sort so it moves into the pending bucket of the nearby list.
	_notify_parent_reorder()

	# Notify the friends panel so the new outgoing request appears in the SENT list live.
	Global.friendship_request_sent.emit(social_data.address)


func _async_on_button_accept_pressed() -> void:
	# Disable appropriate buttons based on which one was clicked
	if item_type == SOCIAL_TYPE.REQUEST:
		button_accept.disabled = true
		button_reject.disabled = true

	var promise = Global.social_service.accept_friend_request(social_data.address)
	await PromiseUtils.async_awaiter(promise)

	if promise.is_rejected():
		printerr("Failed to accept friend request: ", PromiseUtils.get_error_message(promise))
		if item_type == SOCIAL_TYPE.REQUEST:
			button_accept.disabled = false
			button_reject.disabled = false
		else:
			button_add_friend.disabled = false
		return

	current_friendship_status = Global.FriendshipStatus.ACCEPTED
	button_add_friend.hide()
	panel_container_request.hide()

	# Accept Friend metric
	Global.metrics.track_accept_friend(social_data.address, social_data.friendship_id)

	# Emit signal locally since the service doesn't stream back our own actions
	Global.social_service.friendship_request_accepted.emit(social_data.address)


func _async_on_button_reject_pressed() -> void:
	# On a sent request this same button acts as "cancel"; on a received one it's "reject".
	if item_type == SOCIAL_TYPE.REQUEST_SENT:
		await _async_cancel_sent_request()
		return
	button_accept.disabled = true
	button_reject.disabled = true
	var promise = Global.social_service.reject_friend_request(social_data.address)
	await PromiseUtils.async_awaiter(promise)

	if promise.is_rejected():
		printerr("Failed to reject friend request: ", PromiseUtils.get_error_message(promise))
		button_accept.disabled = false
		button_reject.disabled = false
		return

	# Wait a frame and small delay to ensure server has processed the rejection
	await get_tree().process_frame
	await get_tree().create_timer(0.2).timeout

	# Update status after rejecting - should be NONE (7) or similar
	# Check the actual status to update UI correctly
	var status_promise = Global.social_service.get_friendship_status(social_data.address)
	await PromiseUtils.async_awaiter(status_promise)

	if not status_promise.is_rejected():
		var status_data = status_promise.get_data()
		var status = status_data.get("status", -1)
		current_friendship_status = status
		_update_button_visibility_from_status()

	# friend_request_reject metric
	Global.metrics.track_click_button("friend_request_reject", "SOCIAL_PANEL", "")

	# Emit signal locally since the service doesn't stream back our own actions
	Global.social_service.friendship_request_rejected.emit(social_data.address)


## Cancels a request WE sent (REQUEST_SENT items) and removes the row from its SENT list.
func _async_cancel_sent_request() -> void:
	button_reject.disabled = true
	var promise = Global.social_service.cancel_friend_request(social_data.address)
	await PromiseUtils.async_awaiter(promise)

	if promise.is_rejected():
		# Recoverable network failure — warn, don't printerr (keeps it out of Sentry).
		push_warning("Failed to cancel friend request: " + PromiseUtils.get_error_message(promise))
		button_reject.disabled = false
		return

	# friend_request_cancel metric
	Global.metrics.track_click_button("friend_request_cancel", "SOCIAL_PANEL", "")

	# Emit locally (the service doesn't echo our own actions): the friends panel drops it from the
	# SENT list and nearby items re-enable their Add Friend button.
	Global.social_service.friendship_request_cancelled.emit(social_data.address)


func _on_button_jump_in_pressed() -> void:
	if not world_name.is_empty():
		# Join via the realm path so we land at the world's spawn, not parcel (0,0).
		Global.async_join_world(world_name)
	elif parcel.size() >= 2:
		var parcel_position = Vector2i(parcel[0], parcel[1])
		Global.async_teleport_to(parcel_position, DclUrls.main_realm())
	else:
		# Benign race: the location can be cleared by a fresh online_locations_changed emit
		# between the button showing and this press. Warn (not push_error → Sentry).
		push_warning("Jump-in pressed with no resolved location")


func _async_fetch_place_data() -> void:
	if parcel.size() < 2:
		return

	var result = await PlacesHelper.async_get_by_position(Vector2i(parcel[0], parcel[1]))

	if not NodeGuard.is_alive(self, "SocialItem._async_fetch_place_data"):
		return

	if result is PromiseError:
		printerr("Error fetching place data: ", result.get_error())
		label_place.text = "Unknown Location"
		return

	var json: Dictionary = result.get_string_response_as_json()

	if json.data.is_empty():
		label_place.text = "Empty parcel"
		# Add to known_locations even if empty to avoid refetching
		var location_entry = {"coord": parcel.duplicate(), "title": "Empty parcel"}
		Global.locations.known_locations.append(location_entry)
	else:
		var place_data = json.data[0]
		var title = place_data.get("title", "interactive-text")
		if title == "interactive-text":
			title = "Unknown Place"

		var location_entry = {"coord": parcel.duplicate(), "title": title}
		# Add to known_locations for future reference
		Global.locations.known_locations.append(location_entry)


func _on_button_unblock_pressed() -> void:
	# user_unblock metric
	Global.metrics.track_click_button("user_unblock", "SOCIAL_PANEL", "")

	# Disable button during RPC call
	var unblock_button = %Button_Unblock
	if unblock_button:
		unblock_button.disabled = true

	_async_unblock_user(social_data.address)


func _async_unblock_user(address: String) -> void:
	var promise = Global.social_service.unblock_user(address)
	await PromiseUtils.async_awaiter(promise)

	if promise.is_rejected():
		var unblock_button = %Button_Unblock
		if unblock_button:
			unblock_button.disabled = false
		var error_msg := PromiseUtils.get_error_message(promise)
		printerr("Unblock failed: ", error_msg)
		NotificationsManager.show_system_toast("Unblock failed", error_msg, "error", "alert")
		return

	Global.social_blacklist.remove_blocked(address)  # Update local cache
	# Update the containing list
	var parent_list = get_parent()
	if parent_list != null and parent_list.has_method("async_update_list"):
		parent_list.async_update_list()
	_notify_other_components_of_change()


func _check_and_update_friend_status() -> void:
	# Check if the address is already a friend
	if not social_data or social_data.address.is_empty():
		return

	# Fetch from server
	_async_check_friend_status_with_loading()


func _async_check_friend_status_with_loading() -> void:
	# While determining button visibility (pending/add friend/friend badge), show skeleton to
	# avoid intermediate UI flicker.
	if Global.player_identity.is_guest or is_guest:
		return
	_set_loading(true)
	await _async_check_friend_status()
	if load_state != LoadState.FAILED:
		_set_loading(false)


func _update_button_visibility_from_status() -> void:
	if Global.player_identity.is_guest or is_guest:
		return
	# Update button and label visibility based on pre-checked friendship status
	if (
		current_friendship_status == Global.FriendshipStatus.REQUEST_SENT
		or current_friendship_status == Global.FriendshipStatus.REQUEST_RECEIVED
	):
		# REQUEST_SENT or REQUEST_RECEIVED - Show pending label, hide button
		button_add_friend.hide()
		panel_container_request.show()

	elif current_friendship_status == Global.FriendshipStatus.ACCEPTED:
		# ACCEPTED - Hide both button and label
		button_add_friend.hide()
		panel_container_request.hide()
	else:
		# NONE, CANCELED, REJECTED, DELETED, or UNKNOWN
		# Show button (re-enabled — a prior send left it disabled), hide the pending label.
		button_add_friend.disabled = false
		button_add_friend.show()
		panel_container_request.hide()


func _async_check_friend_status() -> void:
	if Global.player_identity.is_guest or is_guest:
		return
	var promise = Global.social_service.get_friendship_status(social_data.address)
	await PromiseUtils.async_awaiter(promise)

	if promise.is_rejected():
		# On error, show the button (default behavior)
		current_friendship_status = Global.FriendshipStatus.UNKNOWN
		button_add_friend.show()
		panel_container_request.hide()
		_notify_parent_reorder()
		return

	var status_data = promise.get_data()
	var status = status_data.get("status", -1)
	current_friendship_status = status

	# Update UI based on status
	_update_button_visibility_from_status()

	# Notify parent to reorder items based on friendship status
	_notify_parent_reorder()


func is_friend() -> bool:
	return current_friendship_status == Global.FriendshipStatus.ACCEPTED


func _notify_parent_reorder() -> void:
	var parent = get_parent()
	if parent and parent.has_method("_request_reorder"):
		parent._request_reorder()


func _on_pressed() -> void:
	# Don't open profile if still loading or no data
	if social_data == null or social_data.address.is_empty():
		return
	Global.open_profile_by_address.emit(social_data.address)


func _on_online_locations_changed() -> void:
	if item_type == SOCIAL_TYPE.ONLINE:
		_update_jump_button_visibility()


func _update_jump_button_visibility() -> void:
	if item_type != SOCIAL_TYPE.ONLINE or not social_data or social_data.address.is_empty():
		button_jump.hide()
		return

	if not Global.locations:
		button_jump.hide()
		return

	label_place.show()
	# Read this friend's location from the shared dict that locations.gd refreshes each cycle
	# (Genesis parcel or World), instead of each item firing its own world request.
	var location = Global.locations.online_locations.get(social_data.address.to_lower(), null)

	if location == null:
		# Not resolved yet this cycle (or online in an unknown place).
		world_name = ""
		parcel.clear()
		label_place.text = "Somewhere"
		button_jump.hide()
		return

	if location.has("world_name"):
		# Friend is in a World.
		parcel.clear()
		world_name = location["world_name"]
		button_jump.show()
		var cached_title: String = Global.locations.known_worlds.get(world_name, "")
		if not cached_title.is_empty():
			label_place.text = cached_title
		else:
			# Show the ENS right away, then resolve the real title once (deduped in locations.gd).
			# Resolution re-emits online_locations_changed, so this row swaps in the title when it
			# lands — no per-row request storm on every emit.
			label_place.text = world_name.trim_suffix(".dcl.eth")
			Global.locations.async_resolve_world_title(world_name)
		return

	# Friend is in Genesis City.
	world_name = ""
	parcel = location["parcel"].duplicate()
	button_jump.show()
	var found_location = false
	for known in Global.locations.known_locations:
		if known.has("coord") and known["coord"].size() >= 2:
			if known["coord"][0] == parcel[0] and known["coord"][1] == parcel[1]:
				if known.has("title"):
					label_place.text = known["title"]
					found_location = true
					break

	if not found_location:
		label_place.text = "Somewhere"
		_async_fetch_place_data()


func _on_blacklist_changed() -> void:
	# Handle blacklist changes for NEARBY and REQUEST items
	_update_blocked_visibility_for_type()


func _update_blocked_visibility_for_type() -> void:
	# Only applies to NEARBY and REQUEST (received/sent) items
	if (
		item_type != SOCIAL_TYPE.NEARBY
		and item_type != SOCIAL_TYPE.REQUEST
		and item_type != SOCIAL_TYPE.REQUEST_SENT
	):
		return

	# Need social_data and address to check
	if not social_data or social_data.address.is_empty():
		return

	# Don't change visibility while loading (item is hidden during loading)
	if _is_loading:
		return

	# Hide if blocked, show if not blocked
	var is_blocked = Global.social_blacklist.is_blocked(social_data.address)
	if is_blocked:
		visible = false
	else:
		visible = true

	# Notify parent to update list size
	_notify_parent_size_changed()


func _set_loading(loading: bool) -> void:
	if not data_container or not skeleton_container:
		return
	data_container.visible = not loading
	skeleton_container.visible = loading
	button_profile.disabled = loading
