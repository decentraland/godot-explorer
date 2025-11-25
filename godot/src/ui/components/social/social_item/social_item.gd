extends Control

enum SocialType { ONLINE, OFFLINE, REQUEST, NEARBY, BLOCKED }

@export var item_type: SocialType

var mute_icon = load("res://assets/ui/audio_off.svg")
var unmute_icon = load("res://assets/ui/audio_on.svg")
var social_data: SocialItemData
var current_friendship_status: int = -1  # -1 = unknown, 0 = REQUEST_SENT, 1 = REQUEST_RECEIVED, 3 = ACCEPTED

@onready var h_box_container_online: HBoxContainer = %HBoxContainer_Online
@onready var h_box_container_nearby: HBoxContainer = %HBoxContainer_Nearby
@onready var h_box_container_request: HBoxContainer = %HBoxContainer_Request
@onready var h_box_container_blocked: HBoxContainer = %HBoxContainer_Blocked
@onready var panel_nearby_player_item: Panel = %Panel_NearbyPlayerItem
@onready var mic_enabled: MarginContainer = %MicEnabled
@onready var nickname: Label = %Nickname
@onready var label_place: Label = %Label_Place
@onready var profile_picture: ProfilePicture = %ProfilePicture
@onready var v_box_container_nickname: VBoxContainer = %VBoxContainer_Nickname
@onready var texture_rect_claimed_checkmark: TextureRect = %TextureRect_ClaimedCheckmark
@onready var button_add_friend: Button = %Button_AddFriend
@onready var button_mute: Button = %Button_Mute
@onready var button_accept: Button = %Button_Accept
@onready var button_reject: Button = %Button_Reject


func _ready():
	add_to_group("blacklist_ui_sync")
	_update_elements_visibility()
	# Connect accept/reject buttons for friend requests
	button_accept.pressed.connect(_async_on_button_accept_pressed)
	button_reject.pressed.connect(_async_on_button_reject_pressed)


func set_data(data: SocialItemData) -> void:
	social_data = data
	var tag_position = data.name.find("#")
	if tag_position != -1:
		data.name = data.name.left(tag_position)
		texture_rect_claimed_checkmark.hide()
	else:
		texture_rect_claimed_checkmark.show()

	if data.name.length() > 15:
		data.name = data.name.left(15) + "..."
	nickname.text = data.name

	var nickname_color = DclAvatar.get_nickname_color(data.name)
	nickname.add_theme_color_override("font_color", nickname_color)
	if data.has_claimed_name:
		texture_rect_claimed_checkmark.show()
	else:
		texture_rect_claimed_checkmark.hide()
	profile_picture.async_update_profile_picture(data)
	
	# If type is NEARBY, check if already a friend
	if item_type == SocialType.NEARBY and not data.address.is_empty():
		_check_and_update_friend_status()


func set_data_from_avatar(avatar_param: DclAvatar):
	social_data = SocialItemData.new()
	social_data.name = avatar_param.get_avatar_name()
	social_data.address = avatar_param.avatar_id
	social_data.profile_picture_url = avatar_param.get_avatar_data().get_snapshots_face_url()
	
	social_data.has_claimed_name = false if social_data.name.contains("#") else true
	set_data(social_data)


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
	if social_data.address == changed_avatar_id:
		call_deferred("_update_buttons")


func _tap_to_open_profile(event: InputEvent) -> void:
	if event is InputEventScreenTouch:
		print(social_data)
		if event.pressed:
			Global.open_profile_by_address.emit(social_data.address)


func _update_elements_visibility() -> void:
	_hide_all_buttons()
	match item_type:
		SocialType.NEARBY:
			h_box_container_nearby.show()
			# Check if already a friend and hide ADD FRIEND button if so
			if social_data and not social_data.address.is_empty():
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


func _get_friends_panel():
	# Navigate up the tree to find the friends panel
	var parent = get_parent()
	while parent != null:
		if parent.has_method("update_all_lists"):
			return parent
		parent = parent.get_parent()
	return null


func set_type(type: SocialType) -> void:
	item_type = type
	_update_elements_visibility()


func _on_button_add_friend_pressed() -> void:
	# Check current status before deciding what to do
	_async_check_status_and_handle_add_friend()


func _async_on_button_add_friend_pressed() -> void:
	button_add_friend.disabled = true
	var promise = Global.social_service.send_friend_request(social_data.address, "")
	await PromiseUtils.async_awaiter(promise)

	if promise.is_rejected():
		printerr("Failed to send friend request: ", promise.get_data().get_error())
		button_add_friend.disabled = false
		return

	button_add_friend.hide()


func _async_on_button_accept_pressed() -> void:
	# Disable appropriate buttons based on which one was clicked
	if item_type == SocialType.REQUEST:
		button_accept.disabled = true
		button_reject.disabled = true
	else:
		# Called from ADD FRIEND button when it says "ACCEPT FRIEND"
		button_add_friend.disabled = true
	
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

	# Hide the button after successful acceptance
	if item_type != SocialType.REQUEST:
		button_add_friend.hide()
		current_friendship_status = 3  # Update to ACCEPTED status

	# Find and update all social lists (especially REQUEST list to remove accepted request)
	var friends_panel = _get_friends_panel()
	if friends_panel and friends_panel.has_method("update_all_lists"):
		friends_panel.update_all_lists()
	else:
		# Fallback: refresh the parent list if friends_panel not found
		var parent_list = get_parent()
		if parent_list and parent_list.has_method("async_update_list"):
			parent_list.async_update_list()

	# Refresh the friends button pending count
	_refresh_friends_button_count()


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

	# Refresh the parent list
	var parent_list = get_parent()
	if parent_list and parent_list.has_method("async_update_list"):
		parent_list.async_update_list()

	# Refresh the friends button pending count
	_refresh_friends_button_count()


func _refresh_friends_button_count() -> void:
	var explorer = Global.get_explorer()
	if explorer and explorer.hud_button_friends:
		explorer.hud_button_friends.refresh_pending_count()


func _on_button_jump_in_pressed() -> void:
	# TODO: Implement teleport to friend location
	pass


func update_location() -> void:
	#var pos = avatar.get_current_parcel_position()
	#var url: String = "https://places.decentraland.org/api/places?limit=1"
	#url += "&positions=%d,%d" % [pos.x, pos.y]
#
	#var headers = {"Content-Type": "application/json"}
	#var promise: Promise = Global.http_requester.request_json(
	#url, HTTPClient.METHOD_GET, "", headers
	#)
	#var result = await PromiseUtils.async_awaiter(promise)
#
	#if result is PromiseError:
	#printerr("Error request places jump in", result.get_error())
	#return
#
	#var json: Dictionary = result.get_string_response_as_json()
#
	#if json.data.is_empty():
	#label_place.text = "Unknown place"
	#else:
	#label_place.text = json.data[0].get("title", "Unknown place")
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


func _async_check_friend_status() -> void:
	var promise = Global.social_service.get_friendship_status(social_data.address)
	await PromiseUtils.async_awaiter(promise)
	
	if promise.is_rejected():
		# On error, show the button (default behavior)
		current_friendship_status = -1
		button_add_friend.show()
		return
	
	var status_data = promise.get_data()
	var status = status_data.get("status", -1)
	current_friendship_status = status
	
	# Status 0 = REQUEST_SENT (we sent a friend request)
	# Status 3 = ACCEPTED (already friends)
	if status == 0 or status == 3:
		button_add_friend.hide()
	elif status == 1:  # REQUEST_RECEIVED (they sent us a request)
		button_add_friend.show()
	else:
		button_add_friend.show()


func _async_check_status_and_handle_add_friend() -> void:
	# Disable button while checking
	button_add_friend.disabled = true
	
	# Check current friendship status
	var promise = Global.social_service.get_friendship_status(social_data.address)
	await PromiseUtils.async_awaiter(promise)
	
	if promise.is_rejected():
		# On error, try to send friend request
		button_add_friend.disabled = false
		_async_on_button_add_friend_pressed()
		return
	
	var status_data = promise.get_data()
	var status = status_data.get("status", -1)
	current_friendship_status = status
	
	# If status is REQUEST_RECEIVED (1), accept the request instead of sending one
	if status == 1:  # REQUEST_RECEIVED
		_async_on_button_accept_pressed()
	else:
		# Otherwise, send a new friend request
		button_add_friend.disabled = false
		_async_on_button_add_friend_pressed()
