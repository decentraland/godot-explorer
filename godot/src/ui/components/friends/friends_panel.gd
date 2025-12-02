extends PanelContainer

signal panel_closed

# ConnectivityStatus enum values from proto
const CONNECTIVITY_ONLINE: int = 0
const CONNECTIVITY_OFFLINE: int = 1
const CONNECTIVITY_AWAY: int = 2

const NO_SERVICE_MESSAGE: String = """Something went wrong and we couldn't retrieve your friends."""
const NO_FRIENDS_MESSAGE: String = """View someone's Profile or tap on 'Add Friend' button in the nearby list."""
const NO_BLOCKED_MESSAGE: String = """If you block someone, you will not be able to see each other in-world or exchange any messages in private or public chats.
You can block another user by going to the tree (3) dots menu available in their Profile."""

var down_arrow_icon: Texture2D = load("res://assets/ui/down_arrow.svg")
var up_arrow_icon: Texture2D = load("res://assets/ui/up_arrow.svg")

# Track which friends are online (address -> true if online)
var _online_friends: Dictionary = {}

@onready var color_rect_friends: ColorRect = %ColorRect_Friends
@onready var color_rect_nearby: ColorRect = %ColorRect_Nearby
@onready var color_rect_blocked: ColorRect = %ColorRect_Blocked
@onready var scroll_container_friends: ScrollContainer = %ScrollContainer_Friends
@onready var scroll_container_nearby: ScrollContainer = %ScrollContainer_Nearby
@onready var scroll_container_blocked: ScrollContainer = %ScrollContainer_Blocked

@onready var request_button: Button = %RequestButton
@onready var online_button: Button = %OnlineButton
@onready var offline_button: Button = %OfflineButton

@onready var v_box_container_request: VBoxContainer = %VBoxContainer_Request
@onready var v_box_container_online: VBoxContainer = %VBoxContainer_Online
@onready var v_box_container_offline: VBoxContainer = %VBoxContainer_Offline
@onready var request_list: VBoxContainer = %RequestList
@onready var online_list: VBoxContainer = %OnlineList
@onready var offline_list: VBoxContainer = %OfflineList
@onready var nearby_list: SocialList = %NearbyList
@onready var blocked_list: SocialList = %BlockedList

@onready var label_empty_state: Label = %LabelEmptyState
@onready var v_box_container_no_service: VBoxContainer = %VBoxContainer_NoService
@onready var v_box_container_no_friends: VBoxContainer = %VBoxContainer_NoFriends
@onready var request_container: PanelContainer = %RequestContainer
@onready var online_container: PanelContainer = %OnlineContainer
@onready var offline_container: PanelContainer = %OfflineContainer
@onready var v_box_container_no_blockeds: VBoxContainer = %VBoxContainer_NoBlockeds
@onready var button_friends: Button = %Button_Friends
@onready var label_no_blockeds: Label = %Label_NoBlockeds
@onready var label_no_friends: Label = %Label_NoFriends
@onready var label_out_of_service: Label = %Label_OutOfService
@onready var timer: Timer = %Timer


func _ready() -> void:
	label_no_blockeds.text = NO_BLOCKED_MESSAGE
	label_no_friends.text = NO_FRIENDS_MESSAGE
	label_out_of_service.text = NO_SERVICE_MESSAGE
	_update_dropdown_visibility()
	_hide_all_drowpdown_highlights()
	_expand_all_friend_lists()
	mouse_filter = Control.MOUSE_FILTER_STOP
	set_process_input(true)
	_on_button_nearby_toggled(true)

	_connect_social_service_signals()

	# Connect to list size changes to update counts
	request_list.size_changed.connect(_update_dropdown_visibility)
	online_list.size_changed.connect(_update_dropdown_visibility)
	offline_list.size_changed.connect(_update_dropdown_visibility)

	# Connect to error signals
	request_list.load_error.connect(_on_load_error)
	online_list.load_error.connect(_on_load_error)


func _connect_social_service_signals() -> void:
	var social = Global.social_service
	_safe_connect(social.friendship_request_received, _on_friendship_request_received)
	_safe_connect(social.friendship_request_accepted, _async_on_friendship_request_accepted)
	_safe_connect(social.friendship_request_rejected, _on_friendship_request_rejected)
	_safe_connect(social.friendship_deleted, _on_friendship_deleted)
	_safe_connect(social.friendship_request_cancelled, _on_friendship_request_cancelled)
	_safe_connect(social.friend_connectivity_updated, _on_friend_connectivity_updated)


func _safe_connect(sig: Signal, callback: Callable) -> void:
	if sig.is_connected(callback):
		sig.disconnect(callback)
	sig.connect(callback)


func _input(event: InputEvent) -> void:
	# Only handle input when panel is visible in tree
	if not is_visible_in_tree():
		return

	# Only process touch events (includes emulated touch from mouse)
	# Ignore mouse events to avoid duplicate processing with emulation enabled
	if not (event is InputEventScreenTouch or event is InputEventScreenDrag):
		return

	# Check if input is within the panel's rectangle
	var pos = event.position
	var rect = get_global_rect()
	var is_inside_panel = rect.has_point(pos)

	# Only release focus on touch press (not during drag) to prevent camera rotation
	# This allows ScrollContainer to handle drag events normally
	if is_inside_panel and event is InputEventScreenTouch and event.pressed:
		if Global.explorer_has_focus():
			Global.explorer_release_focus()


func show_panel_on_friends_tab() -> void:
	show()
	_load_unloaded_items()
	_hide_all_drowpdown_highlights()
	# Switch to friends tab by setting the button pressed (triggers _on_button_friends_toggled)
	button_friends.button_pressed = true


func hide_panel() -> void:
	hide()


func _hide_all() -> void:
	color_rect_friends.self_modulate = Color.TRANSPARENT
	color_rect_nearby.self_modulate = Color.TRANSPARENT
	color_rect_blocked.self_modulate = Color.TRANSPARENT
	scroll_container_nearby.hide()
	scroll_container_friends.hide()
	scroll_container_blocked.hide()


func _on_button_friends_toggled(toggled_on: bool) -> void:
	if toggled_on:
		_hide_all()
		color_rect_friends.self_modulate = Color.WHITE
		scroll_container_friends.show()
		_expand_all_friend_lists()


func _on_button_nearby_toggled(toggled_on: bool) -> void:
	if toggled_on:
		_hide_all()
		color_rect_nearby.self_modulate = Color.WHITE
		scroll_container_nearby.show()


func _on_button_blocked_toggled(toggled_on: bool) -> void:
	if toggled_on:
		_hide_all()
		color_rect_blocked.self_modulate = Color.WHITE
		scroll_container_blocked.show()
		_check_blocked_list_size()


func _on_offline_button_toggled(toggled_on: bool) -> void:
	if toggled_on:
		offline_button.icon = up_arrow_icon
		offline_list.show()
	else:
		offline_button.icon = down_arrow_icon
		offline_list.hide()


func _update_dropdown_visibility() -> void:
	# Check if user is a guest - guests don't have access to friends service
	var is_guest = Global.player_identity.is_guest

	# Only show service error if we explicitly got an error from the lists
	# Don't show error just because connection is still being established
	var has_service_error = not is_guest and (request_list.has_error or online_list.has_error)

	var pending_count = request_list.list_size
	var online_count = online_list.list_size
	var offline_count = offline_list.list_size
	var total_friends = online_count + offline_count

	if pending_count == 0:
		v_box_container_request.hide()
	else:
		v_box_container_request.show()
		request_button.text = "REQUESTS (" + str(pending_count) + ")"

	if online_count == 0:
		v_box_container_online.hide()
	else:
		v_box_container_online.show()
		online_button.text = "ONLINE (" + str(online_count) + ")"

	if offline_count == 0:
		v_box_container_offline.hide()
	else:
		v_box_container_offline.show()
		offline_button.text = "OFFLINE (" + str(offline_count) + ")"

	# Show error message only if we got explicit errors from the lists
	if has_service_error:
		v_box_container_no_service.show()
		v_box_container_no_friends.hide()
	elif total_friends == 0 and pending_count == 0 and not is_guest:
		v_box_container_no_service.hide()
		v_box_container_no_friends.show()
	else:
		v_box_container_no_service.hide()
		v_box_container_no_friends.hide()


func _on_load_error(_error_message: String) -> void:
	# Update visibility to show error state
	_update_dropdown_visibility()


func _on_friendship_request_received(address: String, message: String) -> void:
	# Add the new request to the list without full refresh
	request_list.async_add_request_by_address(address)
	# Update dropdown visibility to show new request count
	_update_dropdown_visibility()
	# Queue a notification for the friend request
	_async_queue_friend_request_notification(address, message)


func _async_queue_friend_request_notification(address: String, message: String) -> void:
	# Fetch the sender's profile to get their display name
	var promise = Global.content_provider.fetch_profile(address)
	var result = await PromiseUtils.async_awaiter(promise)

	var sender_name = address  # Fallback to address if profile fetch fails
	if not (result is PromiseError):
		var profile = result as DclUserProfile
		if profile != null:
			sender_name = profile.get_name()

	# Queue the notification
	NotificationsManager.queue_friend_request_notification(address, sender_name, message)


func _async_on_friendship_request_accepted(address: String) -> void:
	# Check if this was someone accepting OUR request (they were not in our request list)
	# vs us accepting THEIR request (they were in our request list)
	var was_incoming_request = request_list.has_item_with_address(address)

	# Remove from request list
	request_list.remove_item_by_address(address)

	# Fetch profile and add to online/offline list
	var item_data = await _async_fetch_friend_profile(address)
	if item_data != null:
		# Check if friend is online and add to appropriate list
		await _async_check_friend_connectivity(address)
		var should_load = visible
		if is_friend_online(address):
			online_list.add_item_by_social_item_data(item_data, should_load)
		else:
			offline_list.add_item_by_social_item_data(item_data, should_load)

		# Only show notification if they accepted OUR request (not us accepting theirs)
		if not was_incoming_request:
			NotificationsManager.queue_friend_accepted_notification(address, item_data.name)


func _on_friendship_request_rejected(address: String) -> void:
	# Remove from request list (we rejected their request)
	request_list.remove_item_by_address(address)


func _on_friendship_deleted(address: String) -> void:
	# Remove from online tracking
	if _online_friends.has(address):
		_online_friends.erase(address)

	# Remove from online/offline lists
	online_list.remove_item_by_address(address)
	offline_list.remove_item_by_address(address)


func _on_friendship_request_cancelled(address: String) -> void:
	# Remove from request list (they cancelled their request to us)
	request_list.remove_item_by_address(address)


func _async_fetch_friend_profile(address: String) -> SocialItemData:
	# Fetch profile for a friend to create SocialItemData
	var promise = Global.content_provider.fetch_profile(address)
	var result = await PromiseUtils.async_awaiter(promise)

	var item = SocialItemData.new()
	item.address = address

	if result is PromiseError:
		# Fallback to address if profile fetch fails
		item.name = address
		item.has_claimed_name = false
		item.profile_picture_url = ""
	else:
		var profile = result as DclUserProfile
		if profile != null:
			item.name = profile.get_name()
			item.has_claimed_name = profile.has_claimed_name()
			item.profile_picture_url = profile.get_avatar().get_snapshots_face_url()
		else:
			item.name = address
			item.has_claimed_name = false
			item.profile_picture_url = ""

	return item


func _on_friend_connectivity_updated(address: String, status: int) -> void:
	# Update our tracking of online friends
	var was_online = _online_friends.has(address)
	var is_now_online = status == CONNECTIVITY_ONLINE

	if is_now_online:
		_online_friends[address] = true
	else:
		_online_friends.erase(address)

	# Move the friend between online/offline lists without full reload
	if was_online and not is_now_online:
		# Friend went offline - move from online to offline list
		var item_data = online_list.get_item_data_by_address(address)
		if item_data != null:
			online_list.remove_item_by_address(address)
			offline_list.add_item_by_social_item_data(item_data)
	elif not was_online and is_now_online:
		# Friend came online - move from offline to online list
		var item_data = offline_list.get_item_data_by_address(address)
		if item_data != null:
			offline_list.remove_item_by_address(address)
			online_list.add_item_by_social_item_data(item_data)
			# Send chat notification that friend came online
			_send_friend_online_chat_message(item_data.name)

	_update_dropdown_visibility()


func _send_friend_online_chat_message(friend_name: String) -> void:
	var nickname_color = DclAvatar.get_nickname_color(friend_name)
	var color_hex = nickname_color.to_html(false)
	var message = (
		"[color=#%s]%s[/color] [color=#8f8]is now online[/color]" % [color_hex, friend_name]
	)
	Global.on_chat_message.emit("system", message, Time.get_unix_time_from_system())


func is_friend_online(address: String) -> bool:
	return _online_friends.has(address)


func _async_check_friend_connectivity(address: String) -> void:
	# Check if a friend is currently online by checking if they're in the nearby avatars
	# This is a fallback for when connectivity updates haven't arrived yet
	var avatars = Global.avatars.get_avatars()
	for avatar in avatars:
		if avatar != null and avatar is Avatar:
			if avatar.avatar_id == address:
				# Friend is nearby, mark as online
				_online_friends[address] = true
				return

	# If friend is not nearby, they might still be online but in a different location
	# The connectivity update signal will handle this when it arrives
	# For now, don't mark as online if not nearby


func update_all_lists():
	request_list.async_update_list()
	online_list.async_update_list()
	offline_list.async_update_list()
	nearby_list.async_update_list()
	blocked_list.async_update_list()


func _load_unloaded_items() -> void:
	request_list.load_unloaded_items()
	online_list.load_unloaded_items()
	offline_list.load_unloaded_items()
	nearby_list.load_unloaded_items()
	blocked_list.load_unloaded_items()


func _on_request_button_pressed() -> void:
	if request_list.visible:
		request_button.icon = down_arrow_icon
		request_list.hide()
	else:
		request_button.icon = up_arrow_icon
		request_list.show()


func _on_online_button_pressed() -> void:
	if online_list.visible:
		online_button.icon = down_arrow_icon
		online_list.hide()
	else:
		online_button.icon = up_arrow_icon
		online_list.show()


func _on_offline_button_pressed() -> void:
	if offline_list.visible:
		offline_button.icon = down_arrow_icon
		offline_list.hide()
	else:
		offline_button.icon = up_arrow_icon
		offline_list.show()


func _hide_all_drowpdown_highlights() -> void:
	request_container.self_modulate = "ffffff00"
	online_container.self_modulate = "ffffff00"
	offline_container.self_modulate = "ffffff00"


func _expand_all_friend_lists() -> void:
	request_list.show()
	request_button.icon = up_arrow_icon
	online_list.show()
	online_button.icon = up_arrow_icon
	offline_list.show()
	offline_button.icon = up_arrow_icon


func _on_offline_button_mouse_entered() -> void:
	offline_container.self_modulate = "#ffffff"


func _on_offline_button_mouse_exited() -> void:
	offline_container.self_modulate = "#ffffff00"


func _on_online_button_mouse_entered() -> void:
	online_container.self_modulate = "#ffffff"


func _on_online_button_mouse_exited() -> void:
	online_container.self_modulate = "#ffffff00"


func _on_request_button_mouse_entered() -> void:
	request_container.self_modulate = "#ffffff"


func _on_request_button_mouse_exited() -> void:
	request_container.self_modulate = "#ffffff00"


func _check_blocked_list_size() -> void:
	if blocked_list.list_size > 0:
		v_box_container_no_blockeds.hide()
	else:
		v_box_container_no_blockeds.show()


func _on_blocked_list_size_changed() -> void:
	_check_blocked_list_size()


func _on_timer_timeout() -> void:
	if visible:
		Global.locations.fetch_peers()
	else:
		timer.stop()


func _on_visibility_changed() -> void:
	Global.locations.fetch_peers()
	if timer:
		if visible:
			timer.start(0)
		else:
			timer.stop()
