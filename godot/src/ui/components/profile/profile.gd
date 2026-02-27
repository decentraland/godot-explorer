extends Control

signal close_profile

const NICK_MAX_LENGTH: int = 15
const MUTE = preload("res://assets/ui/audio_off.svg")
const UNMUTE = preload("res://assets/ui/audio_on.svg")
const BLOCK = preload("res://assets/ui/block.svg")

@export var rounded: bool = false
@export var closable: bool = false

var url_to_visit: String = ""
var avatar_loading_counter: int = 0
var is_own_passport: bool = false
var is_blocked_user: bool = false
var is_muted_user: bool = false
var current_profile: DclUserProfile = null
var current_friendship_status: int = Global.FriendshipStatus.UNKNOWN
var address: String = ""
var player_profile = Global.player_identity.get_profile_or_null()
var _deploy_loading_id: int = -1
var _deploy_timeout_timer: Timer

@onready var scroll_container: ScrollContainer = %ScrollContainer
@onready var avatar_preview: AvatarPreview = %AvatarPreview
@onready var profile_about: VBoxContainer = %ProfileAbout
@onready var profile_equipped: VBoxContainer = %ProfileEquipped
@onready var profile_links: VBoxContainer = %ProfileLinks
@onready var label_nickname: Label = %Label_Nickname
@onready var label_address: Label = %Label_Address
@onready var texture_rect_claimed_checkmark: TextureRect = %TextureRect_ClaimedCheckmark
@onready var label_tag: Label = %Label_Tag
@onready var button_add_friend: CustomButton = %Button_AddFriend
@onready var button_pending: CustomButton = %Button_Pending
@onready var button_block_user: Button = %Button_BlockUser
@onready var url_popup: ColorRect = %UrlPopup
@onready var profile_new_link_popup: ColorRect = %ProfileNewLinkPopup
@onready var change_nick_popup: ColorRect = %ChangeNickPopup
@onready var v_box_container_content: VBoxContainer = %VBoxContainer_Content
@onready var panel_container_getting_data: PanelContainer = %PanelContainer_GettingData
@onready var button_mute_user: Button = %Button_MuteUser
@onready var button_edit_profile: Button = %Button_EditProfile
@onready var button_close_profile: Button = %Button_CloseProfile
@onready var button_menu: Button = %Button_Menu
@onready var button_cancel_request: Button = %Button_CancelRequest
@onready var button_friend: Button = %Button_Friend
@onready var button_unfriend: Button = %Button_Unfriend
@onready var button_unmute_user: Button = %Button_UnmuteUser
@onready var button_unblock_user: Button = %Button_UnblockUser
@onready var menu: ColorRect = %Menu
@onready var mutual_friends: Control = %MutualFriends
@onready var profile_header: VBoxContainer = %ProfileHeader
@onready var control_own_and_landscape: Control = %Control_OwnAndLandscape
@onready var control_own_and_landscape_menu: VBoxContainer = %Control_OwnAndLandscapeMenu


func _ready() -> void:
	scroll_container.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	Global.player_identity.profile_changed.connect(self._on_global_profile_changed)
	button_menu.button_pressed = false
	menu.hide()

	if rounded:
		var current_style = get_theme_stylebox("panel")
		if current_style is StyleBoxFlat:
			var style_box = current_style.duplicate()
			style_box.corner_radius_top_left = 15
			style_box.corner_radius_top_right = 15
			style_box.corner_radius_bottom_right = 15
			style_box.corner_radius_bottom_left = 15
			add_theme_stylebox_override("panel", style_box)

	if closable:
		button_close_profile.show()
		control_own_and_landscape.hide()
		control_own_and_landscape_menu.hide()
	else:
		button_close_profile.hide()
		control_own_and_landscape.show()
		control_own_and_landscape_menu.show()

	_deploy_timeout_timer = Timer.new()
	_deploy_timeout_timer.wait_time = 15.0
	_deploy_timeout_timer.one_shot = true
	_deploy_timeout_timer.timeout.connect(self._async_on_deploy_timeout)
	add_child(_deploy_timeout_timer)

	_update_elements_visibility()
	add_to_group("blacklist_ui_sync")

	profile_equipped.emote_pressed.connect(_on_emote_pressed)
	profile_equipped.stop_emote.connect(_on_stop_emote)
	profile_links.link_clicked.connect(_open_go_to_link)
	button_edit_profile.pressed.connect(_on_button_edit_profile_pressed)

	# Connect friendship buttons
	if not button_add_friend.pressed.is_connected(_on_button_add_friend_pressed):
		button_add_friend.pressed.connect(_on_button_add_friend_pressed)
	if not button_cancel_request.pressed.is_connected(_on_button_cancel_request_pressed):
		button_cancel_request.pressed.connect(_on_button_cancel_request_pressed)
	if not button_unfriend.pressed.is_connected(_on_button_unfriend_pressed):
		button_unfriend.pressed.connect(_on_button_unfriend_pressed)
	if not button_block_user.pressed.is_connected(_on_button_block_user_pressed):
		button_block_user.pressed.connect(_on_button_block_user_pressed)
	if not button_unblock_user.pressed.is_connected(_on_button_unblock_user_pressed):
		button_unblock_user.pressed.connect(_on_button_unblock_user_pressed)
	if not button_mute_user.pressed.is_connected(_on_button_mute_user_pressed):
		button_mute_user.pressed.connect(_on_button_mute_user_pressed)
	if not button_unmute_user.pressed.is_connected(_on_button_unmute_user_pressed):
		button_unmute_user.pressed.connect(_on_button_unmute_user_pressed)

	# Connect to blacklist changes to update button states
	if not Global.social_blacklist.blacklist_changed.is_connected(
		_on_blacklist_changed_for_buttons
	):
		Global.social_blacklist.blacklist_changed.connect(_on_blacklist_changed_for_buttons)


func _update_elements_visibility() -> void:
	# Hide all friendship buttons by default - they will be shown by _update_friendship_buttons()
	button_add_friend.hide()
	button_cancel_request.hide()
	button_friend.hide()
	button_unfriend.hide()
	url_popup.close()
	change_nick_popup.close()
	profile_new_link_popup.close()
	menu.hide()
	if is_own_passport:
		button_block_user.hide()
		button_mute_user.hide()
		button_cancel_request.hide()
		button_friend.hide()
		button_menu.hide()
		button_add_friend.hide()
		button_unfriend.hide()
		button_edit_profile.show()
	else:
		button_block_user.show()
		button_mute_user.show()
		button_menu.show()
		button_edit_profile.hide()

	if current_profile != null:
		if current_profile.has_claimed_name():
			texture_rect_claimed_checkmark.show()
			label_tag.text = ""
			label_tag.hide()
		else:
			texture_rect_claimed_checkmark.hide()
			label_tag.show()
			label_tag.text = "#" + address.substr(address.length() - 4, 4)


func _set_avatar_loading() -> int:
	panel_container_getting_data.show()
	profile_header.hide()
	v_box_container_content.hide()
	avatar_preview.hide()
	avatar_loading_counter += 1
	return avatar_loading_counter


func _unset_avatar_loading(current: int):
	if current != avatar_loading_counter:
		return
	avatar_preview.show()
	panel_container_getting_data.hide()
	profile_header.show()
	v_box_container_content.show()
	_on_stop_emote()
	if not avatar_preview.avatar.emote_controller.is_playing():
		avatar_preview.avatar.emote_controller.async_play_emote("wave")
	_update_elements_visibility()
	_update_buttons()
	_update_friendship_buttons()


func async_show_profile(profile: DclUserProfile) -> void:
	_hide_all_social_buttons()
	profile_about.hide()
	current_profile = profile
	# Reset friendship status to ensure buttons don't show with old state
	current_friendship_status = Global.FriendshipStatus.UNKNOWN
	await avatar_preview.avatar.async_update_avatar_from_profile(current_profile)

	if player_profile != null:
		is_own_passport = profile.get_ethereum_address() == player_profile.get_ethereum_address()
	else:
		is_own_passport = false

	var loading_id := _set_avatar_loading()

	profile_about.refresh(current_profile)
	profile_links.refresh(current_profile)
	_refresh_name_and_address()
	profile_equipped.async_refresh(current_profile)

	change_nick_popup.close()
	profile_new_link_popup.close()
	url_popup.close()

	_unset_avatar_loading(loading_id)

	if not is_own_passport:
		_connect_friendship_signals()
		# Wait for friendship status check before showing buttons
		await _async_check_friendship_status()
		mutual_friends.async_set_mutual_friends(profile.get_ethereum_address())

	if is_own_passport:
		var mutable: DclUserProfile = Global.player_identity.get_mutable_profile()
		if mutable != null and profile.get_profile_version() < mutable.get_profile_version():
			if _deploy_loading_id == -1:
				_deploy_loading_id = _set_avatar_loading()
				_deploy_timeout_timer.start()

	UiSounds.play_sound("mainmenu_widget_open")
	show()


func _on_emote_pressed(urn: String) -> void:
	avatar_preview.reset_avatar_rotation()
	avatar_preview.avatar.emote_controller.stop_emote()
	if not avatar_preview.avatar.emote_controller.is_playing():
		avatar_preview.avatar.emote_controller.async_play_emote(urn)


func _on_stop_emote() -> void:
	avatar_preview.avatar.emote_controller.stop_emote()


func _on_reset_avatars_rotation() -> void:
	avatar_preview.reset_avatar_rotation()


func close() -> void:
	hide()
	_hide_all_social_buttons()
	_on_stop_emote()
	_on_reset_avatars_rotation()
	_disconnect_friendship_signals()
	if closable:
		close_profile.emit()


func _on_button_edit_nick_pressed() -> void:
	change_nick_popup.open()


func _refresh_name_and_address() -> void:
	address = current_profile.get_ethereum_address()
	label_address.text = Global.shorten_address(address)

	label_nickname.text = current_profile.get_name()
	var nickname_color = DclAvatar.get_nickname_color(current_profile.get_name())
	label_nickname.add_theme_color_override("font_color", nickname_color)


func _open_go_to_link(link_url: String) -> void:
	url_popup.open(link_url)


func _async_on_change_nick_popup_update_name_on_profile(nickname: String) -> void:
	label_nickname.text = nickname
	Global.player_identity.get_mutable_profile().set_name(nickname)
	await Global.player_identity.async_save_profile_metadata()


func _copy_name_and_tag() -> void:
	DisplayServer.clipboard_set(label_nickname.text + label_tag.text)


func _copy_address() -> void:
	DisplayServer.clipboard_set(address)


func _on_label_nickname_gui_input(event: InputEvent) -> void:
	if event is InputEventScreenTouch:
		if event.pressed:
			_copy_name_and_tag()


func _on_label_tag_gui_input(event: InputEvent) -> void:
	if event is InputEventScreenTouch:
		if event.pressed:
			_copy_name_and_tag()


func _on_label_address_gui_input(event: InputEvent) -> void:
	if event is InputEventScreenTouch:
		if event.pressed:
			_copy_address()


func _on_global_profile_changed(new_profile: DclUserProfile) -> void:
	if new_profile == null:
		return
	var new_addr = new_profile.get_ethereum_address()
	if not is_own_passport and new_addr != address:
		return
	current_profile = new_profile
	profile_links.refresh(current_profile)
	profile_about.refresh(current_profile)
	_refresh_name_and_address()
	if _deploy_loading_id != -1:
		_unset_avatar_loading(_deploy_loading_id)
		_deploy_loading_id = -1
	if _deploy_timeout_timer != null and _deploy_timeout_timer.is_stopped() == false:
		_deploy_timeout_timer.stop()


func _async_on_deploy_timeout() -> void:
	if _deploy_loading_id == -1:
		return
	var addr = Global.player_identity.get_address_str()
	var lambda_url = Global.realm.get_lambda_server_base_url()
	await Global.player_identity.async_fetch_profile(addr, lambda_url)
	if _deploy_loading_id != -1:
		_unset_avatar_loading(_deploy_loading_id)
		_deploy_loading_id = -1


func _on_button_mute_user_toggled(toggled_on: bool) -> void:
	if toggled_on:
		Global.social_blacklist.add_muted(avatar_preview.avatar.avatar_id)
	else:
		Global.social_blacklist.remove_muted(avatar_preview.avatar.avatar_id)
	_update_buttons()

	_notify_other_components_of_change()


func _check_block_and_mute_status() -> void:
	var current_avatar = avatar_preview.avatar
	is_blocked_user = Global.social_blacklist.is_blocked(current_avatar.avatar_id)
	is_muted_user = Global.social_blacklist.is_muted(current_avatar.avatar_id)

	if is_blocked_user:
		button_block_user.hide()
		button_mute_user.hide()
	elif is_muted_user:
		button_block_user.show()
		button_mute_user.button_pressed = true


func _update_buttons() -> void:
	if is_own_passport:
		return
	var current_avatar = avatar_preview.avatar
	is_blocked_user = Global.social_blacklist.is_blocked(current_avatar.avatar_id)
	is_muted_user = Global.social_blacklist.is_muted(current_avatar.avatar_id)
	if is_blocked_user:
		button_block_user.hide()
		button_unblock_user.show()
		button_mute_user.hide()
		button_unmute_user.hide()
	else:
		button_block_user.show()
		button_unblock_user.hide()
		if is_muted_user:
			button_mute_user.hide()
			button_unmute_user.show()
		else:
			button_mute_user.show()
			button_unmute_user.hide()

	# Update friendship buttons based on status (only if status has been checked)
	# Don't update if status is still UNKNOWN and we haven't verified it yet
	if current_friendship_status != Global.FriendshipStatus.UNKNOWN or is_own_passport:
		_update_friendship_buttons()


func _async_block_user(user_address: String) -> void:
	var promise = Global.social_service.block_user(user_address)
	await PromiseUtils.async_awaiter(promise)
	button_block_user.disabled = false

	if promise.is_rejected():
		printerr("Block failed: ", PromiseUtils.get_error_message(promise))
		return

	# Block User metric (track whether blocked user was a friend)
	var was_friend := current_friendship_status == Global.FriendshipStatus.ACCEPTED
	Global.metrics.track_block_user(user_address, was_friend)

	Global.social_blacklist.add_blocked(user_address)  # Update local cache
	current_friendship_status = Global.FriendshipStatus.NONE
	_hide_friendship_buttons()
	_update_buttons()
	_notify_other_components_of_change()
	_async_delete_friendship_if_exists(user_address)


func _async_unblock_user_from_profile(user_address: String) -> void:
	var promise = Global.social_service.unblock_user(user_address)
	await PromiseUtils.async_awaiter(promise)
	button_unblock_user.disabled = false

	if promise.is_rejected():
		printerr("Unblock failed: ", PromiseUtils.get_error_message(promise))
		return

	Global.social_blacklist.remove_blocked(user_address)  # Update local cache
	_notify_other_components_of_change()
	_update_buttons()
	_async_update_buttons_and_lists()


func _notify_other_components_of_change() -> void:
	if avatar_preview.avatar != null:
		Global.get_tree().call_group(
			"blacklist_ui_sync", "_sync_blacklist_ui", avatar_preview.avatar.avatar_id
		)


func _sync_blacklist_ui(changed_avatar_id: String) -> void:
	if (
		not is_own_passport
		and current_profile != null
		and avatar_preview.avatar != null
		and avatar_preview.avatar.avatar_id == changed_avatar_id
	):
		call_deferred("_update_buttons")


func _on_blacklist_changed_for_buttons() -> void:
	# Update friendship button disabled states when blacklist changes
	if not is_own_passport and current_profile != null:
		call_deferred("_update_friendship_buttons")


func _on_button_close_profile_button_up() -> void:
	close()


func _on_visibility_changed() -> void:
	if visible:
		grab_focus()


func _async_delete_friendship_if_exists(friend_address: String) -> void:
	# Check if there's an active friendship or pending request
	var promise = Global.social_service.get_friendship_status(friend_address)
	await PromiseUtils.async_awaiter(promise)

	if promise.is_rejected():
		# On error, skip deletion
		return

	var status_data = promise.get_data()
	var status = status_data.get("status", -1)

	var action_promise = null

	# Handle different relationship statuses
	match status:
		Global.FriendshipStatus.REQUEST_SENT:
			action_promise = Global.social_service.cancel_friend_request(friend_address)
		Global.FriendshipStatus.REQUEST_RECEIVED:
			action_promise = Global.social_service.reject_friend_request(friend_address)
		Global.FriendshipStatus.ACCEPTED:
			action_promise = Global.social_service.delete_friendship(friend_address)
		_:  # No relationship or other status, nothing to do
			return

	if action_promise != null:
		await PromiseUtils.async_awaiter(action_promise)

		if action_promise.is_rejected():
			printerr(
				"Failed to remove relationship when blocking: ",
				action_promise.get_data().get_error()
			)
		_async_update_buttons_and_lists()


func _get_friends_panel():
	var parent = get_parent()
	while parent != null:
		if parent.has_method("update_all_lists") and parent.has_method("is_friend_online"):
			print("Profile: Found friends panel in parent tree")
			return parent
		parent = parent.get_parent()
	var scene_tree = get_tree()
	if scene_tree == null:
		return null
	var friends_panel = _find_friends_panel_recursive(scene_tree.root)
	if friends_panel != null:
		print("Profile: Found friends panel in scene tree")
	return friends_panel


func _find_friends_panel_recursive(node: Node) -> Node:
	if node == null:
		return null
	if node.has_method("update_all_lists") and node.has_method("is_friend_online"):
		return node
	for child in node.get_children():
		var result = _find_friends_panel_recursive(child)
		if result != null:
			return result
	return null


func _force_update_all_social_lists() -> void:
	var scene_tree = get_tree()
	if scene_tree == null:
		return
	_force_update_social_lists_recursive(scene_tree.root)


func _force_update_social_lists_recursive(node: Node) -> void:
	if node == null:
		return
	if node.has_method("async_update_list") and "player_list_type" in node:
		var list_type = node.get("player_list_type")
		if list_type in [0, 1, 2, 3]:
			node.call_deferred("async_update_list")
	for child in node.get_children():
		_force_update_social_lists_recursive(child)


func _on_button_menu_toggled(toggled_on: bool) -> void:
	if toggled_on:
		menu.show()
	else:
		menu.hide()


func _connect_friendship_signals() -> void:
	if is_own_passport:
		return

	# Connect to friendship status change signals
	if not Global.social_service.friendship_request_received.is_connected(
		_on_friendship_request_received
	):
		Global.social_service.friendship_request_received.connect(_on_friendship_request_received)
	if not Global.social_service.friendship_request_accepted.is_connected(
		_on_friendship_request_accepted
	):
		Global.social_service.friendship_request_accepted.connect(_on_friendship_request_accepted)
	if not Global.social_service.friendship_request_rejected.is_connected(
		_on_friendship_request_rejected
	):
		Global.social_service.friendship_request_rejected.connect(_on_friendship_request_rejected)
	if not Global.social_service.friendship_deleted.is_connected(_on_friendship_deleted):
		Global.social_service.friendship_deleted.connect(_on_friendship_deleted)
	if not Global.social_service.friendship_request_cancelled.is_connected(
		_on_friendship_request_cancelled
	):
		Global.social_service.friendship_request_cancelled.connect(_on_friendship_request_cancelled)


func _disconnect_friendship_signals() -> void:
	# Disconnect all friendship signals
	if Global.social_service.friendship_request_received.is_connected(
		_on_friendship_request_received
	):
		Global.social_service.friendship_request_received.disconnect(
			_on_friendship_request_received
		)
	if Global.social_service.friendship_request_accepted.is_connected(
		_on_friendship_request_accepted
	):
		Global.social_service.friendship_request_accepted.disconnect(
			_on_friendship_request_accepted
		)
	if Global.social_service.friendship_request_rejected.is_connected(
		_on_friendship_request_rejected
	):
		Global.social_service.friendship_request_rejected.disconnect(
			_on_friendship_request_rejected
		)
	if Global.social_service.friendship_deleted.is_connected(_on_friendship_deleted):
		Global.social_service.friendship_deleted.disconnect(_on_friendship_deleted)
	if Global.social_service.friendship_request_cancelled.is_connected(
		_on_friendship_request_cancelled
	):
		Global.social_service.friendship_request_cancelled.disconnect(
			_on_friendship_request_cancelled
		)


func _on_friendship_request_received(friend_address: String, _message: String = "") -> void:
	_handle_friendship_change(friend_address, Global.FriendshipStatus.REQUEST_RECEIVED)


func _on_friendship_request_accepted(friend_address: String) -> void:
	_handle_friendship_change(friend_address, Global.FriendshipStatus.ACCEPTED)


func _on_friendship_request_rejected(friend_address: String) -> void:
	_handle_friendship_change(friend_address, Global.FriendshipStatus.NONE)


func _on_friendship_deleted(friend_address: String) -> void:
	_handle_friendship_change(friend_address, Global.FriendshipStatus.NONE)


func _on_friendship_request_cancelled(friend_address: String) -> void:
	_handle_friendship_change(friend_address, Global.FriendshipStatus.NONE)


func _handle_friendship_change(friend_address: String, new_status: int) -> void:
	if current_profile == null:
		return
	if current_profile.get_ethereum_address().to_lower() != friend_address.to_lower():
		return
	current_friendship_status = new_status
	_update_friendship_buttons()
	_update_friend_lists()


func _on_button_cancel_request_pressed() -> void:
	if is_own_passport or current_profile == null:
		return

	var friend_address = current_profile.get_ethereum_address()
	_async_cancel_friend_request(friend_address)


func _async_cancel_friend_request(friend_address: String) -> void:
	var promise = Global.social_service.cancel_friend_request(friend_address)
	await PromiseUtils.async_awaiter(promise)

	if promise.is_rejected():
		printerr("Failed to cancel friend request: ", promise.get_data().get_error())
		return

	_async_update_buttons_and_lists()


func _on_button_unfriend_pressed() -> void:
	if is_own_passport or current_profile == null:
		return
	button_menu.button_pressed = false
	var friend_address = current_profile.get_ethereum_address()
	_async_unfriend(friend_address)


func _async_unfriend(friend_address: String) -> void:
	print("Profile: _async_unfriend called for address: ", friend_address)
	var promise = Global.social_service.delete_friendship(friend_address)
	await PromiseUtils.async_awaiter(promise)

	if promise.is_rejected():
		printerr("Failed to unfriend: ", promise.get_data().get_error())
		return

	# Unfriend metric
	Global.metrics.track_unfriend(friend_address)

	print("Profile: Unfriend successful, waiting for signal to update lists")
	# The signal friendship_deleted will update the UI
	# But also update immediately to ensure UI is responsive
	_async_update_buttons_and_lists()


func _on_button_add_friend_pressed() -> void:
	if is_own_passport or current_profile == null:
		return
	var friend_address = current_profile.get_ethereum_address()
	if current_friendship_status == Global.FriendshipStatus.REQUEST_RECEIVED:
		_async_accept_friend_request(friend_address)
	else:
		_async_send_friend_request(friend_address)


func _async_send_friend_request(friend_address: String) -> void:
	button_add_friend.hide()
	button_pending.show()
	var promise = Global.social_service.send_friend_request(friend_address, "")
	await PromiseUtils.async_awaiter(promise)

	if promise.is_rejected():
		printerr("Failed to send friend request: ", promise.get_data().get_error())
		button_pending.hide()
		button_add_friend.show()
		return

	# Request Friend metric
	Global.metrics.track_request_friend(friend_address)

	_async_update_buttons_and_lists()


func _async_accept_friend_request(friend_address: String) -> void:
	button_add_friend.disabled = true
	var promise = Global.social_service.accept_friend_request(friend_address)
	await PromiseUtils.async_awaiter(promise)
	button_add_friend.disabled = false

	if promise.is_rejected():
		printerr("Failed to accept friend request: ", promise.get_data().get_error())
		return

	# Accept Friend metric (no friendship_id available in profile context)
	Global.metrics.track_accept_friend(friend_address, "")

	_async_update_buttons_and_lists()


func _async_check_friendship_status() -> void:
	if is_own_passport or current_profile == null:
		return

	# Check if social service is available before making the call
	if not _is_social_service_available():
		current_friendship_status = Global.FriendshipStatus.UNKNOWN
		_update_friendship_buttons()
		return

	var friend_address = current_profile.get_ethereum_address()
	print("Profile: Checking friendship status for address: ", friend_address)
	var promise = Global.social_service.get_friendship_status(friend_address)
	await PromiseUtils.async_awaiter(promise)

	if promise.is_rejected():
		# On error, service might not be available or there was an error
		# Hide all friendship buttons
		print("Profile: Friendship status check failed: ", promise.get_data().get_error())
		current_friendship_status = Global.FriendshipStatus.UNKNOWN
		_update_friendship_buttons()
		return

	var status_data = promise.get_data()
	current_friendship_status = status_data.get("status", Global.FriendshipStatus.UNKNOWN)
	print("Profile: Friendship status result: ", current_friendship_status)
	_update_friendship_buttons()


func _update_friendship_buttons() -> void:
	if is_own_passport or not _is_social_service_available():
		return
	_hide_friendship_buttons()

	# Guest users cannot have social interactions
	if Global.player_identity.is_guest:
		return

	# Check if target user is a guest (hasn't connected web3)
	if current_profile != null and not current_profile.has_connected_web3():
		return

	# Check if user is blocked - if blocked, don't show any friendship buttons
	var current_avatar = avatar_preview.avatar
	var is_user_blocked = false
	if current_avatar != null and not current_avatar.avatar_id.is_empty():
		is_user_blocked = Global.social_blacklist.is_blocked(current_avatar.avatar_id)

	# If user is blocked, hide all friendship buttons
	if is_user_blocked:
		return

	match current_friendship_status:
		Global.FriendshipStatus.ACCEPTED:
			button_friend.show()
			button_friend.button_pressed = true
			button_unfriend.show()
		Global.FriendshipStatus.REQUEST_SENT:
			button_cancel_request.show()
		Global.FriendshipStatus.REQUEST_RECEIVED:
			button_add_friend.show()
			button_add_friend.custom_text = "ACCEPT"
		_:  # NONE, UNKNOWN, or other statuses
			if not is_blocked_user:
				button_add_friend.show()
				button_add_friend.custom_text = "ADD FRIEND"


func _is_social_service_available() -> bool:
	return Global.social_service != null


func _async_update_buttons_and_lists():
	await _async_check_friendship_status()
	_update_friend_lists()


func _update_friend_lists() -> void:
	var friends_panel = _get_friends_panel()
	if friends_panel != null and friends_panel.has_method("update_all_lists"):
		friends_panel.update_all_lists()
	else:
		_force_update_all_social_lists()


func _hide_all_social_buttons() -> void:
	_hide_friendship_buttons()
	mutual_friends.hide()


func _hide_friendship_buttons() -> void:
	button_add_friend.hide()
	button_pending.hide()
	button_cancel_request.hide()
	button_friend.hide()
	button_unfriend.hide()


func _on_button_edit_profile_pressed() -> void:
	close()
	Global.set_orientation_portrait()
	Global.open_profile_editor.emit()


func _on_copy_nick_pressed() -> void:
	_copy_name_and_tag()


func _on_copy_address_pressed() -> void:
	_copy_address()


func _on_button_block_user_pressed() -> void:
	var current_avatar = avatar_preview.avatar
	button_block_user.disabled = true
	Global.metrics.track_click_button("user_block", "PROFILE", "")
	_async_block_user(current_avatar.avatar_id)


func _on_button_unblock_user_pressed() -> void:
	var current_avatar = avatar_preview.avatar
	button_unblock_user.disabled = true
	Global.metrics.track_click_button("user_unblock", "PROFILE", "")
	_async_unblock_user_from_profile(current_avatar.avatar_id)


func _on_button_mute_user_pressed() -> void:
	Global.social_blacklist.add_muted(avatar_preview.avatar.avatar_id)
	call_deferred("_update_buttons")
	_notify_other_components_of_change()


func _on_button_unmute_user_pressed() -> void:
	Global.social_blacklist.remove_muted(avatar_preview.avatar.avatar_id)
	call_deferred("_update_buttons")
	_notify_other_components_of_change()
