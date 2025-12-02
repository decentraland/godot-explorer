extends Button

enum SocialType { ONLINE, OFFLINE, REQUEST, NEARBY, BLOCKED }
enum LoadState { UNLOADED, LOADING, LOADED }

@export var item_type: SocialType

var mute_icon = load("res://assets/ui/audio_off.svg")
var unmute_icon = load("res://assets/ui/audio_on.svg")
var social_data: SocialItemData
var current_friendship_status: int = Global.FriendshipStatus.UNKNOWN
var load_state: LoadState = LoadState.UNLOADED
var _avatar_ref: WeakRef = null  # Weak reference to avatar for nearby items
var _is_loading: bool = false

@onready var h_box_container_online: HBoxContainer = %HBoxContainer_Online
@onready var h_box_container_nearby: HBoxContainer = %HBoxContainer_Nearby
@onready var h_box_container_request: HBoxContainer = %HBoxContainer_Request
@onready var h_box_container_blocked: HBoxContainer = %HBoxContainer_Blocked
@onready var panel_nearby_player_item: Panel = %Panel_NearbyPlayerItem
@onready var nickname: Label = %Nickname
@onready var label_place: Label = %Label_Place
@onready var profile_picture: ProfilePicture = %ProfilePicture
@onready var v_box_container_nickname: VBoxContainer = %VBoxContainer_Nickname
@onready var texture_rect_claimed_checkmark: TextureRect = %TextureRect_ClaimedCheckmark
@onready var button_add_friend: Button = %Button_AddFriend
@onready var button_mute: Button = %Button_Mute
@onready var button_accept: Button = %Button_Accept
@onready var button_reject: Button = %Button_Reject
@onready var label_pending_request: Label = %Label_PendingRequest


func _ready():
	add_to_group("blacklist_ui_sync")
	_update_elements_visibility()
	# Connect accept/reject buttons for friend requests
	button_accept.pressed.connect(_async_on_button_accept_pressed)
	button_reject.pressed.connect(_async_on_button_reject_pressed)


func set_data(data: SocialItemData, should_load: bool = true) -> void:
	social_data = data
	_apply_data_to_ui()

	if should_load:
		load_item()
	else:
		load_state = LoadState.UNLOADED


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

	if display_name.length() > 15:
		display_name = display_name.left(15) + "..."
	nickname.text = display_name

	var nickname_color = DclAvatar.get_nickname_color(social_data.name)
	nickname.add_theme_color_override("font_color", nickname_color)
	if social_data.has_claimed_name:
		texture_rect_claimed_checkmark.show()
	else:
		texture_rect_claimed_checkmark.hide()


func load_item() -> void:
	if load_state == LoadState.LOADED or load_state == LoadState.LOADING:
		return
	if social_data == null:
		return

	load_state = LoadState.LOADING
	profile_picture.async_update_profile_picture(social_data)
	load_state = LoadState.LOADED

	# If type is NEARBY, check if already a friend
	if item_type == SocialType.NEARBY and not social_data.address.is_empty():
		_update_buttons()
		_check_and_update_friend_status()


func set_data_from_avatar(avatar_param: Avatar) -> void:
	_avatar_ref = weakref(avatar_param)

	# Hide self initially while loading
	visible = false
	_is_loading = true

	# If avatar is not ready, wait for it
	if not avatar_param.avatar_ready:
		avatar_param.avatar_loaded.connect(_on_avatar_loaded, CONNECT_ONE_SHOT)
		return

	# Avatar is ready, load data immediately
	_load_data_from_avatar(avatar_param)


func _on_avatar_loaded() -> void:
	var avatar = _avatar_ref.get_ref() as Avatar if _avatar_ref else null
	if avatar == null or not is_instance_valid(avatar):
		# Avatar was freed, remove self
		queue_free()
		return

	_load_data_from_avatar(avatar)


func _load_data_from_avatar(avatar_param: Avatar) -> void:
	# Check if avatar_id is set (it should be after avatar_ready)
	if avatar_param.avatar_id.is_empty():
		# Still no avatar_id, remove self
		queue_free()
		return

	social_data = SocialItemData.new()
	social_data.name = avatar_param.get_avatar_name()
	social_data.address = avatar_param.avatar_id
	social_data.profile_picture_url = avatar_param.get_avatar_data().get_snapshots_face_url()

	social_data.has_claimed_name = false if social_data.name.contains("#") else true

	# Now show self and set data
	_is_loading = false
	visible = true
	set_data(social_data)

	# Notify parent that we're ready (for list size updates)
	_notify_parent_size_changed()


func _on_mouse_entered() -> void:
	panel_nearby_player_item.self_modulate = "#ffffff"


func _on_mouse_exited() -> void:
	panel_nearby_player_item.self_modulate = "#ffffff00"


func _on_button_mute_toggled(toggled_on: bool) -> void:
	if toggled_on:
		Global.social_blacklist.add_muted(social_data.address)
	else:
		Global.social_blacklist.remove_muted(social_data.address)
	_update_buttons()
	_notify_other_components_of_change()


func _update_buttons() -> void:
	var is_muted = Global.social_blacklist.is_muted(social_data.address)
	button_mute.set_pressed_no_signal(is_muted)
	if is_muted:
		button_mute.icon = mute_icon
	else:
		button_mute.icon = unmute_icon


func _notify_other_components_of_change() -> void:
	if social_data.address:
		Global.get_tree().call_group("blacklist_ui_sync", "_sync_blacklist_ui", social_data.address)


func _sync_blacklist_ui(changed_avatar_id: String) -> void:
	if social_data and social_data.address == changed_avatar_id:
		call_deferred("_update_buttons")


func _update_elements_visibility() -> void:
	_hide_all_buttons()
	match item_type:
		SocialType.NEARBY:
			h_box_container_nearby.show()
			# Check if already a friend and hide/show ADD FRIEND button accordingly
			if social_data and not social_data.address.is_empty():
				_update_buttons()
				# If status is already known (pre-checked), use it directly
				if current_friendship_status != Global.FriendshipStatus.UNKNOWN:
					_update_button_visibility_from_status()
				else:
					# Hide button initially to avoid flickering, will show/hide after checking status
					button_add_friend.hide()
					_check_and_update_friend_status()
		SocialType.ONLINE:
			h_box_container_online.show()
			profile_picture.set_online()
		SocialType.OFFLINE:
			h_box_container_online.show()
			profile_picture.set_offline()
		SocialType.REQUEST:
			h_box_container_request.show()
		SocialType.BLOCKED:
			h_box_container_blocked.show()
		_:
			profile_picture.hide_status()


func _hide_all_buttons() -> void:
	h_box_container_online.hide()
	h_box_container_nearby.hide()
	h_box_container_request.hide()
	h_box_container_blocked.hide()
	profile_picture.hide_status()
	label_place.hide()
	button_add_friend.hide()
	label_pending_request.hide()


func _notify_parent_size_changed() -> void:
	var parent = get_parent()
	if parent and parent.has_signal("size_changed"):
		parent.size_changed.emit()


func set_type(type: SocialType) -> void:
	item_type = type
	_update_elements_visibility()


func _on_button_add_friend_pressed() -> void:
	# ADD FRIEND button only sends friend requests (original behavior)
	_async_on_button_add_friend_pressed()


func _async_on_button_add_friend_pressed() -> void:
	button_add_friend.disabled = true
	var promise = Global.social_service.send_friend_request(social_data.address, "")
	await PromiseUtils.async_awaiter(promise)

	if promise.is_rejected():
		printerr("Failed to send friend request: ", promise.get_data().get_error())
		button_add_friend.disabled = false
		return

	current_friendship_status = Global.FriendshipStatus.REQUEST_SENT
	button_add_friend.hide()
	label_pending_request.show()


func _async_on_button_accept_pressed() -> void:
	# Disable appropriate buttons based on which one was clicked
	if item_type == SocialType.REQUEST:
		button_accept.disabled = true
		button_reject.disabled = true

	var promise = Global.social_service.accept_friend_request(social_data.address)
	await PromiseUtils.async_awaiter(promise)

	if promise.is_rejected():
		printerr("Failed to accept friend request: ", promise.get_data().get_error())
		if item_type == SocialType.REQUEST:
			button_accept.disabled = false
			button_reject.disabled = false
		else:
			button_add_friend.disabled = false
		return

	current_friendship_status = Global.FriendshipStatus.ACCEPTED
	button_add_friend.hide()
	label_pending_request.hide()
	_refresh_friends_button_count()

	# Emit signal locally since the service doesn't stream back our own actions
	Global.social_service.friendship_request_accepted.emit(social_data.address)


func _async_on_button_reject_pressed() -> void:
	button_accept.disabled = true
	button_reject.disabled = true
	var promise = Global.social_service.reject_friend_request(social_data.address)
	await PromiseUtils.async_awaiter(promise)

	if promise.is_rejected():
		printerr("Failed to reject friend request: ", promise.get_data().get_error())
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

	_refresh_friends_button_count()

	# Emit signal locally since the service doesn't stream back our own actions
	Global.social_service.friendship_request_rejected.emit(social_data.address)


func _refresh_friends_button_count() -> void:
	var explorer = Global.get_explorer()
	if explorer and explorer.hud_button_friends:
		explorer.hud_button_friends.refresh_pending_count()


func _on_button_jump_in_pressed() -> void:
	# TODO: Implement teleport to friend location
	pass


func update_location() -> void:
	pass


func _on_button_unblock_pressed() -> void:
	Global.social_blacklist.remove_blocked(social_data.address)
	# Actualizar la lista contenedora
	var parent_list = get_parent()
	if parent_list != null and parent_list.has_method("async_update_list"):
		parent_list.async_update_list()
	_notify_other_components_of_change()


func _check_and_update_friend_status() -> void:
	# Check if the address is already a friend
	if not social_data or social_data.address.is_empty():
		return

	_async_check_friend_status()


func _update_button_visibility_from_status() -> void:
	# Update button and label visibility based on pre-checked friendship status
	if (
		current_friendship_status == Global.FriendshipStatus.REQUEST_SENT
		or current_friendship_status == Global.FriendshipStatus.REQUEST_RECEIVED
	):
		# REQUEST_SENT or REQUEST_RECEIVED - Show pending label, hide button
		button_add_friend.hide()
		label_pending_request.show()
	elif current_friendship_status == Global.FriendshipStatus.ACCEPTED:
		# ACCEPTED - Hide both button and label
		button_add_friend.hide()
		label_pending_request.hide()
		profile_picture.set_friend()
	else:
		# NONE, CANCELED, REJECTED, DELETED, or UNKNOWN
		# Show button, hide label (can send new request)
		button_add_friend.show()
		label_pending_request.hide()


func _async_check_friend_status() -> void:
	var promise = Global.social_service.get_friendship_status(social_data.address)
	await PromiseUtils.async_awaiter(promise)

	if promise.is_rejected():
		# On error, show the button (default behavior)
		current_friendship_status = Global.FriendshipStatus.UNKNOWN
		button_add_friend.show()
		label_pending_request.hide()
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
	Global.open_profile_by_address.emit(social_data.address)
