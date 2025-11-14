class_name Discover
extends Control

var search_text: String = ""

@onready var jump_in: SidePanelWrapper = %JumpIn
@onready var event_details: SidePanelWrapper = %EventDetails

@onready var button_search_bar: Button = %Button_SearchBar
@onready var line_edit_search_bar: LineEdit = %LineEdit_SearchBar
@onready var button_clear_filter: Button = %Button_ClearFilter
@onready var timer_search_debounce: Timer = %Timer_SearchDebounce

@onready var last_visited: VBoxContainer = %LastVisited
@onready var places_featured: VBoxContainer = %PlacesFeatured
@onready var places_most_active: VBoxContainer = %PlacesMostActive
@onready var places_worlds: VBoxContainer = %PlacesWorlds
@onready var events: VBoxContainer = %Events


func _ready():
	UiSounds.install_audio_recusirve(self)
	jump_in.hide()
	event_details.hide()
	button_search_bar.show()
	button_clear_filter.hide()
	line_edit_search_bar.hide()

	# Connect to notification clicked signal
	Global.notification_clicked.connect(_on_notification_clicked)

	# Initialize and test social service
	_init_social_service()


func on_item_pressed(data):
	jump_in.set_data(data)
	jump_in.show_animation()


func on_event_pressed(data):
	event_details.set_data(data)
	event_details.show_animation()


func _on_jump_in_jump_in(parcel_position: Vector2i, realm: String):
	jump_in.hide()
	Global.teleport_to(parcel_position, realm)


func _on_visibility_changed():
	if is_node_ready() and is_inside_tree() and is_visible_in_tree():
		%LastVisitGenerator.request_last_places()


func _on_line_edit_search_bar_focus_exited() -> void:
	button_search_bar.show()
	line_edit_search_bar.hide()


func _on_button_search_bar_pressed() -> void:
	button_search_bar.hide()
	line_edit_search_bar.show()
	line_edit_search_bar.grab_focus()


func _on_button_clear_filter_pressed() -> void:
	search_text = ""
	set_search_filter_text("")
	line_edit_search_bar.text = ""
	timer_search_debounce.stop()


func set_search_filter_text(new_text: String) -> void:
	button_clear_filter.visible = !new_text.is_empty()

	if new_text.is_empty():
		last_visited.show()
		places_featured.show()
	else:
		last_visited.hide()
		places_featured.hide()
	places_most_active.set_search_param(new_text)
	places_worlds.set_search_param(new_text)
	events.set_search_param(new_text)


func _on_line_edit_search_bar_text_changed(new_text: String) -> void:
	search_text = new_text
	timer_search_debounce.start()


func _on_timer_search_debounce_timeout() -> void:
	set_search_filter_text(search_text)


func _on_event_details_jump_in(parcel_position: Vector2i, realm: String) -> void:
	event_details.hide()
	Global.teleport_to(parcel_position, realm)


func _on_notification_clicked(notification: Dictionary) -> void:
	# Handle notification clicks - open event details for event notifications
	var notif_type = notification.get("type", "")

	# Early return if not an event notification
	if notif_type not in ["event_created", "events_starts_soon", "events_started"]:
		return

	var metadata = notification.get("metadata", {})

	# Extract event ID from the link URL (e.g., "https://decentraland.org/jump/events?id=5f776ddc-...")
	var link = metadata.get("link", "")
	if link.is_empty():
		printerr("[Discover] Event notification missing link in metadata")
		return

	# Parse event ID from URL query parameter
	var event_id = _extract_event_id_from_url(link)
	if event_id.is_empty():
		printerr("[Discover] Could not extract event ID from link: ", link)
		return

	# Fetch event data and show event details
	_async_handle_event_notification(event_id)


func _extract_event_id_from_url(url: String) -> String:
	# Extract event ID from URL like "https://decentraland.org/jump/events?id=5f776ddc-bcc9-49e5-aa2c-d84f0b5dda27"
	var query_start = url.find("?")
	if query_start == -1:
		return ""

	var query_string = url.substr(query_start + 1)
	var params = query_string.split("&")

	for param in params:
		var key_value = param.split("=")
		if key_value.size() == 2 and key_value[0] == "id":
			return key_value[1]

	return ""


func _async_handle_event_notification(event_id: String) -> void:
	# Fetch event data from API
	var url = "https://events.decentraland.org/api/events/" + event_id
	var response = await Global.async_signed_fetch(url, HTTPClient.METHOD_GET, "")

	if response is PromiseError:
		printerr("[Discover] Failed to fetch event data: ", response.get_error())
		return

	var json: Dictionary = response.get_string_response_as_json()

	if not json.has("data"):
		printerr("[Discover] Invalid event response format")
		return

	var event_data = json["data"]

	# Show event details
	on_event_pressed(event_data)


# Friends system integration
func _init_social_service() -> void:
	print("[Discover] Initializing Social Service...")

	# Create DclSocialService node
	var social_service = DclSocialService.new()
	add_child(social_service)

	# Initialize with player identity wallet
	social_service.initialize_from_player_identity(Global.player_identity)

	# Connect to friendship update signals
	social_service.friendship_request_received.connect(_on_friend_request_received)
	social_service.friendship_request_accepted.connect(_on_friend_request_accepted)
	social_service.friendship_request_rejected.connect(_on_friend_request_rejected)
	social_service.friendship_deleted.connect(_on_friendship_deleted)
	social_service.friendship_request_cancelled.connect(_on_friend_request_cancelled)

	# Wait a frame for async initialization to complete
	await get_tree().process_frame

	# Fetch and print friends list
	_fetch_friends(social_service)

	# Fetch and print pending friend requests
	_fetch_pending_requests(social_service)

	# Subscribe to real-time updates
	_subscribe_to_updates(social_service)


func _fetch_friends(social_service: DclSocialService) -> void:
	print("[Discover] Fetching friends list...")

	# Get friends (limit: 50, offset: 0, status: -1 for ALL)
	var promise = social_service.get_friends(50, 0, -1)

	# Wait for promise to resolve
	await promise.on_resolved

	if promise.is_rejected():
		var error = promise.get_data()
		printerr("[Discover] Failed to get friends: ", error.get_error())
		return

	var friends = promise.get_data()
	print("[Discover] ✅ Friends list (", friends.size(), " friends):")
	for friend in friends:
		print("  - ", friend)


func _fetch_pending_requests(social_service: DclSocialService) -> void:
	print("[Discover] Fetching pending friend requests...")

	# Get pending requests (limit: 50, offset: 0)
	var promise = social_service.get_pending_requests(50, 0)

	# Wait for promise to resolve
	await promise.on_resolved

	if promise.is_rejected():
		var error = promise.get_data()
		printerr("[Discover] Failed to get pending requests: ", error.get_error())
		return

	var requests = promise.get_data()
	print("[Discover] ✅ Pending friend requests (", requests.size(), " requests):")
	for request in requests:
		print("  - From: ", request.address)
		print("    Message: ", request.message if request.message else "(no message)")
		print("    Created at: ", request.created_at)


func _subscribe_to_updates(social_service: DclSocialService) -> void:
	print("[Discover] Subscribing to friendship updates...")

	var promise = social_service.subscribe_to_updates()

	await promise.on_resolved

	if promise.is_rejected():
		var error = promise.get_data()
		printerr("[Discover] Failed to subscribe to updates: ", error.get_error())
		return

	print("[Discover] ✅ Subscribed to real-time friendship updates")


# Signal handlers for real-time friendship updates
func _on_friend_request_received(address: String, message: String) -> void:
	print("[Discover] 🔔 New friend request from: ", address)
	if message:
		print("  Message: ", message)


func _on_friend_request_accepted(address: String) -> void:
	print("[Discover] 🎉 Friend request accepted by: ", address)


func _on_friend_request_rejected(address: String) -> void:
	print("[Discover] ❌ Friend request rejected by: ", address)


func _on_friendship_deleted(address: String) -> void:
	print("[Discover] 💔 Friendship deleted with: ", address)


func _on_friend_request_cancelled(address: String) -> void:
	print("[Discover] 🚫 Friend request cancelled by: ", address)
