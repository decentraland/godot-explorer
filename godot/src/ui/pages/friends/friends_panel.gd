extends PanelContainer

signal panel_closed

# ConnectivityStatus enum values from proto
const CONNECTIVITY_ONLINE: int = 0
const CONNECTIVITY_OFFLINE: int = 1
const CONNECTIVITY_AWAY: int = 2

var down_arrow_icon: Texture2D = load("res://assets/ui/down_arrow.svg")
var up_arrow_icon: Texture2D = load("res://assets/ui/up_arrow.svg")

# Track which friends are online (address -> true if online)
var _online_friends: Dictionary = {}

# Track if streaming subscription failed (to show service error and retry on panel open)
# Starts as true (service down) until the service loads successfully
var _streaming_subscription_failed: bool = true

# Track if we're currently loading friends data
var _is_loading: bool = false

@onready var color_rect_friends: ColorRect = %ColorRect_Friends
@onready var color_rect_requests: ColorRect = %ColorRect_Requests
@onready var color_rect_nearby: ColorRect = %ColorRect_Nearby

@onready var scroll_container_friends: ScrollContainer = %ScrollContainer_Friends
@onready var scroll_container_requests: ScrollContainer = %ScrollContainer_Requests
@onready var scroll_container_nearby: ScrollContainer = %ScrollContainer_Nearby

# FRIENDS tab: Online / Offline / Blocked collapsible sections.
@onready var online_button: Button = %OnlineButton
@onready var offline_button: Button = %OfflineButton
@onready var blocked_button: Button = %BlockedButton
@onready var online_container: PanelContainer = %OnlineContainer
@onready var offline_container: PanelContainer = %OfflineContainer
@onready var blocked_container: PanelContainer = %BlockedContainer
@onready var v_box_container_online: VBoxContainer = %VBoxContainer_Online
@onready var v_box_container_offline: VBoxContainer = %VBoxContainer_Offline
@onready var v_box_container_blocked: VBoxContainer = %VBoxContainer_Blocked
@onready var online_list: SocialList = %OnlineList
@onready var offline_list: SocialList = %OfflineList
@onready var blocked_list: SocialList = %BlockedList
@onready var label_online_count: Label = %Label_OnlineCount
@onready var label_offline_count: Label = %Label_OfflineCount
@onready var label_blocked_count: Label = %Label_BlockedCount

# REQUESTS tab: Received / Sent collapsible sections. `request_list` holds the RECEIVED (incoming)
# requests — the friendship signal handlers all operate on it; `sent_list` holds outgoing ones.
@onready var received_button: Button = %ReceivedButton
@onready var sent_button: Button = %SentButton
@onready var v_box_container_received: VBoxContainer = %VBoxContainer_Received
@onready var v_box_container_sent: VBoxContainer = %VBoxContainer_Sent
@onready var request_list: SocialList = %ReceivedList
@onready var sent_list: SocialList = %SentList
@onready var label_received_count: Label = %Label_ReceivedCount
@onready var label_sent_count: Label = %Label_SentCount

@onready var nearby_list: SocialList = %NearbyList

@onready var v_box_container_no_service: VBoxContainer = %VBoxContainer_NoService
@onready var v_box_container_loading: VBoxContainer = %VBoxContainer_Loading
@onready var label_out_of_service: Label = %Label_OutOfService
@onready var friends_list: VBoxContainer = %FriendsList

@onready var button_friends: Button = %Button_Friends
@onready var button_requests: Button = %Button_Requests
@onready var button_nearby: Button = %Button_Nearby
@onready var timer: Timer = %Timer

@onready var h_box_container_friends_tab: HBoxContainer = %HBoxContainer_FriendsTab
@onready var h_box_container_requests_tab: HBoxContainer = %HBoxContainer_RequestsTab
@onready var h_box_container_nearby_tab: HBoxContainer = %HBoxContainer_NearbyTab

@onready var margin_container_no_nearby: MarginContainer = %MarginContainer_NoNearby
@onready var margin_container_no_requests: MarginContainer = %MarginContainer_NoRequests
@onready var margin_container_no_friends: MarginContainer = %MarginContainer_NoFriends


func _ready() -> void:
	# The section title + count come from the child labels (Label / Label_*Count), so the
	# buttons themselves carry no text.
	online_button.text = ""
	offline_button.text = ""
	blocked_button.text = ""
	received_button.text = ""
	sent_button.text = ""
	_hide_all_drowpdown_highlights()
	mouse_filter = Control.MOUSE_FILTER_STOP
	set_process_input(true)
	toggle_nearby()

	_connect_social_service_signals()

	# FRIENDS tab dropdown counts (Online / Offline / Blocked).
	online_list.size_changed.connect(_update_dropdown_visibility)
	offline_list.size_changed.connect(_update_dropdown_visibility)
	blocked_list.size_changed.connect(_update_dropdown_visibility)
	# REQUESTS tab: Received drives the navbar badge; both sections drive the tab layout.
	request_list.size_changed.connect(_update_requests)
	request_list.size_changed.connect(_update_badge_visibility_on_navbar)
	sent_list.size_changed.connect(_update_requests)
	received_button.pressed.connect(_on_received_button_pressed)
	sent_button.pressed.connect(_on_sent_button_pressed)
	# NEARBY tab.
	nearby_list.size_changed.connect(_on_nearby_list_size_changed)

	# Connect to error signals
	request_list.load_error.connect(_on_load_error)
	online_list.load_error.connect(_on_load_error)

	# Initial state: hide all containers - will show loading when panel opens
	v_box_container_online.hide()
	v_box_container_offline.hide()
	v_box_container_blocked.hide()
	margin_container_no_friends.hide()
	v_box_container_no_service.hide()
	v_box_container_loading.hide()
	friends_list.hide()
	_on_nearby_list_size_changed()
	_update_requests()


func _connect_social_service_signals() -> void:
	var social = Global.social_service
	_safe_connect(social.friendship_request_received, _on_friendship_request_received)
	_safe_connect(social.friendship_request_accepted, _async_on_friendship_request_accepted)
	_safe_connect(social.friendship_request_rejected, _on_friendship_request_rejected)
	_safe_connect(social.friendship_deleted, _on_friendship_deleted)
	_safe_connect(social.friendship_request_cancelled, _on_friendship_request_cancelled)
	_safe_connect(social.friend_connectivity_updated, _on_friend_connectivity_updated)
	# Our own sent requests aren't streamed by the service — Global relays them locally.
	_safe_connect(Global.friendship_request_sent, _on_friendship_request_sent)


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


## Opens the friends panel
func show_panel_on_friends_tab() -> void:
	show()
	_load_unloaded_items()
	_hide_all_drowpdown_highlights()
	if not Global.player_identity.is_guest:
		h_box_container_friends_tab.show()
		button_friends.button_pressed = true
	else:
		h_box_container_friends_tab.hide()
		button_nearby.button_pressed = true


func hide_panel() -> void:
	hide()
	panel_closed.emit()


func set_streaming_subscription_failed(failed: bool) -> void:
	if failed and not _streaming_subscription_failed:
		printerr("[FriendsPanel.SubscriptionState] flipped failed=true (warning visible to user)")
	_streaming_subscription_failed = failed
	_update_dropdown_visibility()


## Fetches all friend lists from the server (called once at login by explorer.gd).
func async_initial_friends_load() -> void:
	if _is_loading:
		return
	_is_loading = true
	_update_dropdown_visibility()
	await _async_update_all_lists()
	_is_loading = false
	_update_dropdown_visibility()


## Diff-based refresh: fetches fresh friends data and syncs lists without full rebuild.
## Used by the proactive re-subscribe timer to avoid UI disruption.
func async_refresh_friends() -> void:
	# Fetch all friends (reusing the same RPC as initial load)
	var promise = Global.social_service.get_friends(100, 0, -1)
	await PromiseUtils.async_awaiter(promise)
	if promise.is_rejected():
		return  # Silent failure — keep showing current data

	var friends = promise.get_data()
	var online_items: Array = []
	var offline_items: Array = []

	for friend_data in friends:
		var item = SocialItemData.new()
		item.address = friend_data["address"]
		item.name = friend_data["name"]
		item.has_claimed_name = friend_data["has_claimed_name"]
		item.profile_picture_url = friend_data["profile_picture_url"]
		if is_friend_online(item.address):
			online_items.append(item)
		else:
			offline_items.append(item)

	online_list.sync_items(online_items)
	offline_list.sync_items(offline_items)

	# Also refresh pending requests
	var req_promise = Global.social_service.get_pending_requests(100, 0)
	await PromiseUtils.async_awaiter(req_promise)
	if not req_promise.is_rejected():
		var requests = req_promise.get_data()
		var request_items: Array = []
		for req in requests:
			var item = SocialItemData.new()
			item.address = req["address"]
			item.name = req["name"]
			item.has_claimed_name = req["has_claimed_name"]
			item.profile_picture_url = req["profile_picture_url"]
			item.friendship_id = req.get("friendship_id", "")
			request_items.append(item)
		request_list.sync_items(request_items)

	_update_dropdown_visibility()


func _hide_all() -> void:
	color_rect_friends.self_modulate = Color.TRANSPARENT
	color_rect_requests.self_modulate = Color.TRANSPARENT
	color_rect_nearby.self_modulate = Color.TRANSPARENT
	scroll_container_friends.hide()
	scroll_container_requests.hide()
	scroll_container_nearby.hide()


func _on_button_friends_toggled(toggled_on: bool) -> void:
	if toggled_on:
		_hide_all()
		color_rect_friends.self_modulate = Color.WHITE
		scroll_container_friends.show()
		_expand_all_friend_lists()


func _on_button_requests_toggled(toggled_on: bool) -> void:
	if toggled_on:
		# requests_menu_opened metric
		Global.metrics.track_click_button("requests_menu_opened", "SOCIAL_PANEL", "")
		_hide_all()
		color_rect_requests.self_modulate = Color.WHITE
		scroll_container_requests.show()
		_expand_request_lists()
		# Received updates live via the friendship_request_received signal; sent requests have no
		# stream (the service doesn't echo our own actions), so re-fetch them on tab open.
		sent_list.async_update_list()
		_update_requests()


func _on_button_nearby_toggled(toggled_on: bool) -> void:
	if toggled_on:
		# nearby_menu_opened metric
		Global.metrics.track_click_button("nearby_menu_opened", "SOCIAL_PANEL", "")
		toggle_nearby()


func toggle_nearby() -> void:
	_hide_all()
	color_rect_nearby.self_modulate = Color.WHITE
	scroll_container_nearby.show()
	_on_nearby_list_size_changed()


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

	# Show loading state if currently loading
	if _is_loading:
		# While loading, show the sections from the start so `social_item` skeletons are visible
		# while each avatar finishes loading.
		v_box_container_loading.hide()
		v_box_container_no_service.hide()
		margin_container_no_friends.hide()
		friends_list.show()
		v_box_container_online.show()
		v_box_container_offline.show()
		# Blocked only shows if there are elements (avoid the "flash" while list_size is 0).
		v_box_container_blocked.visible = blocked_list.list_size > 0
		return

	# Hide loading container when not loading
	v_box_container_loading.hide()

	# Show service error if streaming subscription failed or lists had errors
	var has_service_error = (
		not is_guest
		and (_streaming_subscription_failed or online_list.has_error or offline_list.has_error)
	)

	var online_count = online_list.list_size
	var offline_count = offline_list.list_size
	var blocked_count = blocked_list.list_size
	var total = online_count + offline_count + blocked_count

	_update_dropdown_section(v_box_container_online, label_online_count, online_count)
	_update_dropdown_section(v_box_container_offline, label_offline_count, offline_count)
	_update_dropdown_section(v_box_container_blocked, label_blocked_count, blocked_count)

	# Show error message only if we got explicit errors from the lists
	if has_service_error:
		v_box_container_no_service.show()
		margin_container_no_friends.hide()
		friends_list.hide()
	elif total == 0 and not is_guest:
		v_box_container_no_service.hide()
		margin_container_no_friends.show()
		friends_list.hide()
	else:
		v_box_container_no_service.hide()
		margin_container_no_friends.hide()
		friends_list.show()


## Shows/hides a FRIENDS-tab section by its count and refreshes its "(n)" count label.
func _update_dropdown_section(section: VBoxContainer, count_label: Label, count: int) -> void:
	if count == 0:
		section.hide()
	else:
		section.show()
		count_label.text = "(%d)" % count


## REQUESTS tab: show/hide the Received and Sent sections by their counts, plus the empty state.
func _update_requests() -> void:
	var received_count = request_list.list_size
	var sent_count = sent_list.list_size
	_update_dropdown_section(v_box_container_received, label_received_count, received_count)
	_update_dropdown_section(v_box_container_sent, label_sent_count, sent_count)
	margin_container_no_requests.visible = received_count + sent_count == 0


func _on_received_button_pressed() -> void:
	if request_list.visible:
		received_button.icon = down_arrow_icon
		request_list.hide()
	else:
		received_button.icon = up_arrow_icon
		request_list.show()


func _on_sent_button_pressed() -> void:
	if sent_list.visible:
		sent_button.icon = down_arrow_icon
		sent_list.hide()
	else:
		sent_button.icon = up_arrow_icon
		sent_list.show()


## Expands both request sections when the REQUESTS tab opens.
func _expand_request_lists() -> void:
	request_list.show()
	received_button.icon = up_arrow_icon
	sent_list.show()
	sent_button.icon = up_arrow_icon


func _update_badge_visibility_on_navbar() -> void:
	Global.friends_request_size_changed.emit(request_list.list_size)


func _on_load_error(_error_message: String) -> void:
	# Update visibility to show error state
	_update_dropdown_visibility()


## A request WE just sent (relayed via Global since the service doesn't stream it): add it to the
## SENT list live so the user sees it without switching tabs or restarting.
func _on_friendship_request_sent(address: String) -> void:
	sent_list.async_add_request_by_address(address)


func _on_friendship_request_received(address: String, _message: String) -> void:
	# Add the new request to the list without full refresh
	request_list.async_add_request_by_address(address)
	# Update dropdown visibility to show new request count
	_update_dropdown_visibility()
	# Trigger fetch_notifications to get the server-side notification sooner
	NotificationsManager.fetch_notifications()


func _async_on_friendship_request_accepted(address: String) -> void:
	# Check if this was someone accepting OUR request (they were not in our request list)
	# vs us accepting THEIR request (they were in our request list)
	var was_incoming_request = request_list.has_item_with_address(address)

	# Remove from both request lists: received (we/they accepted) or sent (they accepted ours).
	request_list.remove_item_by_address(address)
	sent_list.remove_item_by_address(address)

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

		# Only fetch notifications if they accepted OUR request (not us accepting theirs)
		if not was_incoming_request:
			NotificationsManager.fetch_notifications()


func _on_friendship_request_rejected(address: String) -> void:
	# We rejected their received request, or they rejected our sent one — drop it from both.
	request_list.remove_item_by_address(address)
	sent_list.remove_item_by_address(address)


func _on_friendship_deleted(address: String) -> void:
	print("FriendsPanel: _on_friendship_deleted called for address: ", address)
	# Remove from online tracking
	if _online_friends.has(address):
		_online_friends.erase(address)

	# Remove from online/offline lists
	var removed_online = online_list.remove_item_by_address(address)
	var removed_offline = offline_list.remove_item_by_address(address)
	# Also drop any pending sent request to this address.
	sent_list.remove_item_by_address(address)
	print("FriendsPanel: Removed from online: ", removed_online, ", offline: ", removed_offline)


func _on_friendship_request_cancelled(address: String) -> void:
	# They cancelled their received request to us, or a sent one was cancelled — drop from both.
	request_list.remove_item_by_address(address)
	sent_list.remove_item_by_address(address)


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
		var item_node = online_list.pop_item_by_address(address)
		if item_node != null:
			offline_list.add_child(item_node)
			item_node.set_type(SocialItemData.SocialType.OFFLINE)
			offline_list._update_list_size()
			online_list._update_list_size()
	elif not was_online and is_now_online:
		# Friend came online - move from offline to online list
		var item_node = offline_list.pop_item_by_address(address)
		if item_node != null:
			online_list.add_child(item_node)
			item_node.set_type(SocialItemData.SocialType.ONLINE)
			online_list._update_list_size()
			offline_list._update_list_size()
			# Send chat notification that friend came online
			if item_node.social_data != null:
				_send_friend_online_chat_message(item_node.social_data.name)

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
	sent_list.async_update_list()
	online_list.async_update_list()
	offline_list.async_update_list()
	nearby_list.async_update_list()
	blocked_list.async_update_list()


func _async_update_all_lists() -> void:
	# Update lists sequentially.
	# Skeleton placeholders are handled in `_update_dropdown_visibility()` to keep UX responsive.
	await request_list.async_update_list()
	sent_list.async_update_list()
	await online_list.async_update_list()
	await offline_list.async_update_list()
	# Don't wait for nearby and blocked as they're not critical for friends tab loading
	nearby_list.async_update_list()
	blocked_list.async_update_list()


func _load_unloaded_items() -> void:
	request_list.load_unloaded_items()
	sent_list.load_unloaded_items()
	online_list.load_unloaded_items()
	offline_list.load_unloaded_items()
	nearby_list.load_unloaded_items()
	blocked_list.load_unloaded_items()


func _on_blocked_button_pressed() -> void:
	if blocked_list.visible:
		blocked_button.icon = down_arrow_icon
		blocked_list.hide()
	else:
		blocked_button.icon = up_arrow_icon
		blocked_list.show()


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
	online_container.self_modulate = "ffffff00"
	offline_container.self_modulate = "ffffff00"
	blocked_container.self_modulate = "ffffff00"


func _expand_all_friend_lists() -> void:
	online_list.show()
	online_button.icon = up_arrow_icon
	offline_list.show()
	offline_button.icon = up_arrow_icon
	blocked_list.show()
	blocked_button.icon = up_arrow_icon


func _on_offline_button_mouse_entered() -> void:
	offline_container.self_modulate = "#ffffff"


func _on_offline_button_mouse_exited() -> void:
	offline_container.self_modulate = "#ffffff00"


func _on_online_button_mouse_entered() -> void:
	online_container.self_modulate = "#ffffff"


func _on_online_button_mouse_exited() -> void:
	online_container.self_modulate = "#ffffff00"


func _on_blocked_button_mouse_entered() -> void:
	blocked_container.self_modulate = "#ffffff"


func _on_blocked_button_mouse_exited() -> void:
	blocked_container.self_modulate = "#ffffff00"


## Refreshes the NEARBY tab. The tab title is static ("NEARBY", set in the scene like FRIENDS /
## REQUESTS — no count), so this only toggles the list with its empty state.
func _on_nearby_list_size_changed() -> void:
	# Empty state vs list (Nearby tab)
	if nearby_list.list_size == 0:
		nearby_list.hide()
		margin_container_no_nearby.show()
	else:
		margin_container_no_nearby.hide()
		nearby_list.show()


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
