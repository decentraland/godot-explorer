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
@onready var events: VBoxContainer = %Events

@onready var search_container := %SearchSuggestionsContainer


func _ready():
	UiSounds.install_audio_recusirve(self)
	jump_in.hide()
	event_details.hide()
	button_search_bar.show()
	button_clear_filter.hide()
	line_edit_search_bar.hide()

	# Connect to notification clicked signal
	Global.notification_clicked.connect(_on_notification_clicked)

	search_container.hide()
	search_container.keyword_selected.connect(_async_on_keyword_selected)

	last_visited.generator.report_loading_status.connect(_on_report_loading_status)
	places_featured.generator.report_loading_status.connect(_on_report_loading_status)
	places_most_active.generator.report_loading_status.connect(_on_report_loading_status)
	events.generator.report_loading_status.connect(_on_report_loading_status)


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
		last_visited.generator.async_request_last_places(0, 10)


func _on_line_edit_search_bar_focus_exited() -> void:
	button_search_bar.show()
	line_edit_search_bar.hide()


func _on_button_search_bar_pressed() -> void:
	button_search_bar.hide()
	line_edit_search_bar.show()
	line_edit_search_bar.grab_focus()
	search_container.show()
	search_container.set_keyword_search_text("")


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
	events.set_search_param(new_text)


func _on_line_edit_search_bar_text_changed(new_text: String) -> void:
	search_text = new_text
	timer_search_debounce.start()


func _on_line_edit_search_bar_text_submitted(new_text: String) -> void:
	var coordinates := {}
	if PlacesHelper.parse_coordinates(new_text, coordinates):
		new_text = await PlacesHelper.async_replace_get_name_from_coordinates(
			Vector2i(coordinates.x, coordinates.y)
		)
	new_text = new_text.lstrip(" .")
	new_text = new_text.rstrip(" .")
	if Global.get_config().add_search_history(new_text):
		Global.get_config().save_to_settings_file()
	search_text = new_text
	set_search_filter_text(search_text)
	search_container.hide()
	button_search_bar.show()
	line_edit_search_bar.hide()
	line_edit_search_bar.text = ""


func _on_timer_search_debounce_timeout() -> void:
	if search_text.length() >= 3:
		search_container.set_keyword_search_text(search_text)
	else:
		search_container.set_keyword_search_text("")


func _on_event_details_jump_in(parcel_position: Vector2i, realm: String) -> void:
	event_details.hide()
	Global.teleport_to(parcel_position, realm)


func _on_notification_clicked(notification_d: Dictionary) -> void:
	# Handle notification clicks - open event details for event notifications
	var notif_type = notification_d.get("type", "")

	# Early return if not an event notification
	if notif_type not in ["event_created", "events_starts_soon", "events_started"]:
		return

	var metadata = notification_d.get("metadata", {})

	# Extract event ID from the link URL (e.g., "https://decentraland.org/jump/events?id=5f776ddc-...")
	var link = metadata.get("link", "")
	if link.is_empty():
		printerr("[Discover] Event notification missing link in metadata")
		_on_error_loading_notification()
		return

	# Parse event ID from URL query parameter
	var event_id = _extract_event_id_from_url(link)
	if event_id.is_empty():
		printerr("[Discover] Could not extract event ID from link: ", link)
		_on_error_loading_notification()
		return

	# Fetch event data and show event details
	_async_handle_event_notification(event_id)
	return


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
		_on_error_loading_notification()
		return

	var json: Dictionary = response.get_string_response_as_json()

	if not json.has("data"):
		printerr("[Discover] Invalid event response format")
		_on_error_loading_notification()
		return

	var event_data = json["data"]

	# Show event details
	on_event_pressed(event_data)


func _on_button_close_pressed() -> void:
	Global.close_menu.emit()


func _on_error_loading_notification() -> void:
	Global.close_navbar.emit()


func _on_report_loading_status(status: CarrouselGenerator.LoadingStatus) -> void:
	match status:
		CarrouselGenerator.LoadingStatus.LOADING:
			%MessageNoResultsFound.show()
		CarrouselGenerator.LoadingStatus.OK_WITH_RESULTS:
			%MessageNoResultsFound.hide()


func _async_on_keyword_selected(keyword: SearchSuggestions.Keyword) -> void:
	var search_keyword := keyword.keyword
	if keyword.type == SearchSuggestions.KeywordType.COORDINATES:
		search_keyword = await PlacesHelper.async_replace_get_name_from_coordinates(
			keyword.coordinates
		)
	set_search_filter_text(search_keyword)
	search_container.hide()
