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


func _ready() -> void:
	_update_dropdown_visibility()
	_hide_all_drowpdown_highlights()
	request_list.hide()
	offline_list.hide()
	# Ensure the panel blocks touch/mouse events from passing through when visible
	mouse_filter = Control.MOUSE_FILTER_STOP
	set_process_input(true)
	_on_button_nearby_toggled(true)

	# Connect to social service signals for real-time updates
	# Check if already connected to avoid duplicate connections
	if not Global.social_service.friendship_request_received.is_connected(
		_on_friendship_request_received
	):
		Global.social_service.friendship_request_received.connect(_on_friendship_request_received)

	# Always reconnect to ensure signals are connected (in case of re-initialization)
	if Global.social_service.friendship_request_accepted.is_connected(_async_on_friendship_changed):
		Global.social_service.friendship_request_accepted.disconnect(_async_on_friendship_changed)
	Global.social_service.friendship_request_accepted.connect(_async_on_friendship_changed)

	if Global.social_service.friendship_request_rejected.is_connected(_async_on_friendship_changed):
		Global.social_service.friendship_request_rejected.disconnect(_async_on_friendship_changed)
	Global.social_service.friendship_request_rejected.connect(_async_on_friendship_changed)

	if Global.social_service.friendship_deleted.is_connected(_async_on_friendship_changed):
		Global.social_service.friendship_deleted.disconnect(_async_on_friendship_changed)
	Global.social_service.friendship_deleted.connect(_async_on_friendship_changed)

	if Global.social_service.friendship_request_cancelled.is_connected(
		_async_on_friendship_changed
	):
		Global.social_service.friendship_request_cancelled.disconnect(_async_on_friendship_changed)
	Global.social_service.friendship_request_cancelled.connect(_async_on_friendship_changed)

	if not Global.social_service.friend_connectivity_updated.is_connected(
		_on_friend_connectivity_updated
	):
		Global.social_service.friend_connectivity_updated.connect(_on_friend_connectivity_updated)

	# Note: Connectivity updates subscription is done in explorer.gd during social service init
	# This ensures the subscription is established before the panel loads

	# Connect to list size changes to update counts
	request_list.size_changed.connect(_update_dropdown_visibility)
	online_list.size_changed.connect(_update_dropdown_visibility)
	offline_list.size_changed.connect(_update_dropdown_visibility)

	# Also connect to friendship_changed to ensure request list updates when requests are accepted/rejected
	# This is already connected above, but we want to make sure request list updates

	# Connect to error signals
	request_list.load_error.connect(_on_load_error)
	online_list.load_error.connect(_on_load_error)


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


func show_panel() -> void:
	show()
	update_all_lists()
	_hide_all_drowpdown_highlights()


func hide_panel() -> void:
	hide()
	update_all_lists()


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


func _on_friendship_request_received(_address: String, _message: String) -> void:
	# Refresh request list when new request arrives
	request_list.async_update_list()
	# Update dropdown visibility to show new request count
	_update_dropdown_visibility()


func _async_on_friendship_changed(address: String) -> void:
	# When a friendship is deleted, remove from online tracking
	if address != "" and _online_friends.has(address):
		_online_friends.erase(address)

	# When a friendship is accepted, check if the friend is online
	# This handles the case where we accept a request from someone who is already online
	if address != "":
		await _async_check_friend_connectivity(address)

	# Refresh all friend lists when friendship status changes
	update_all_lists()

	# Also update dropdown visibility to reflect new counts
	await get_tree().process_frame
	_update_dropdown_visibility()


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

	_update_dropdown_visibility()


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
