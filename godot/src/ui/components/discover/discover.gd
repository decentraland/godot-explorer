class_name Discover
extends Control

var search_text: String = ""
var _generator_statuses: Dictionary = {}

@onready var jump_in: SidePanelWrapper = %JumpIn
@onready var event_details: SidePanelWrapper = %EventDetails

@onready var search_bar: SearchBar = %SearchBar
@onready var timer_search_debounce: Timer = %Timer_SearchDebounce

@onready var last_visited: VBoxContainer = %LastVisited
@onready var places_featured: VBoxContainer = %PlacesFeatured
@onready var places_most_active: VBoxContainer = %PlacesMostActive
@onready var events: VBoxContainer = %Events
@onready var places_favorites: VBoxContainer = %PlacesFavorites
@onready var places_my_places: VBoxContainer = %PlacesMyPlaces
@onready var search_container: SearchSuggestions = %SearchSugestionsContainer
@onready var button_back_to_explorer: Button = %Button_BackToExplorer
@onready var label_title: Label = %Label_Title
@onready var container_content: ScrollRubberContainer = %ScrollContainer_Content


func _ready():
	UiSounds.install_audio_recusirve(self)
	button_back_to_explorer.hide()
	jump_in.hide()
	event_details.hide()
	search_bar.close_searchbar()

	jump_in.jump_in.connect(_on_jump_in_jump_in)
	jump_in.jump_in_world.connect(_on_jump_in_world)
	event_details.jump_in.connect(_on_event_details_jump_in)
	event_details.jump_in_world.connect(_on_event_details_jump_in_world)

	Global.notification_clicked.connect(_on_notification_clicked)

	search_container.hide()
	search_container.keyword_selected.connect(_async_on_keyword_selected)
	search_container.should_show_container.connect(_on_should_show_suggestions_container)
	search_bar.cleared.connect(_on_search_bar_cleared)
	container_content.show()

	last_visited.generator.report_loading_status.connect(
		_on_report_loading_status.bind(last_visited)
	)
	places_featured.generator.report_loading_status.connect(
		_on_report_loading_status.bind(places_featured)
	)
	places_most_active.generator.report_loading_status.connect(
		_on_report_loading_status.bind(places_most_active)
	)
	events.generator.report_loading_status.connect(_on_report_loading_status.bind(events))
	places_favorites.generator.report_loading_status.connect(
		_on_report_loading_status.bind(places_favorites)
	)


func on_item_pressed(data):
	jump_in.set_data(data)
	jump_in.open_panel()


func on_event_pressed(data):
	if data is String:
		_async_handle_event_notification(data)
		return
	event_details.set_data(data)
	event_details.open_panel()


func _on_jump_in_jump_in(parcel_position: Vector2i, realm: String):
	jump_in.hide()
	Global.teleport_to(parcel_position, realm)


func _on_jump_in_world(realm: String):
	jump_in.hide()
	Global.join_world(realm)


func _on_visibility_changed():
	if is_node_ready() and is_inside_tree() and is_visible_in_tree():
		last_visited.generator.async_request_last_places(0, 10)
		Global.set_orientation_portrait()
		if Global.get_explorer():
			if button_back_to_explorer:
				button_back_to_explorer.show()


func _on_search_bar_opened() -> void:
	button_back_to_explorer.show()
	label_title.hide()
	search_container.show()
	container_content.hide()
	search_container.set_keyword_search_text("")


func _on_search_bar_cleared() -> void:
	search_text = ""
	set_search_filter_text("")
	timer_search_debounce.stop()
	search_container.hide()
	container_content.show()


func set_search_filter_text(new_text: String) -> void:
	_generator_statuses.clear()
	if new_text.is_empty():
		last_visited.visible = last_visited.has_items()
		places_featured.show()
		places_my_places.visible = places_my_places.has_items()
	else:
		last_visited.hide()
		places_my_places.hide()
	places_featured.set_search_param(new_text)
	places_most_active.set_search_param(new_text)
	events.set_search_param(new_text)
	places_favorites.set_search_param(new_text)
	_scroll_all_carousels_to_start()


func _scroll_all_carousels_to_start() -> void:
	container_content.reset_position()
	for carousel in [
		places_featured,
		events,
		last_visited,
		places_most_active,
		places_favorites,
		places_my_places
	]:
		if carousel.has_method("scroll_to_start"):
			carousel.scroll_to_start()


func _on_line_edit_search_bar_text_changed(new_text: String) -> void:
	search_text = new_text
	if not new_text.is_empty() and not search_container.visible:
		search_container.show()
		container_content.hide()
	search_container.set_keyword_search_text(search_text)


func _on_should_show_suggestions_container(should_show: bool) -> void:
	if should_show:
		search_container.show()
		container_content.hide()
	else:
		search_container.hide()
		container_content.show()


func _async_on_line_edit_search_bar_text_submitted(new_text: String) -> void:
	var coordinates := {}
	if PlacesHelper.parse_coordinates(new_text, coordinates):
		new_text = await PlacesHelper.async_get_name_from_coordinates(
			Vector2i(coordinates.x, coordinates.y)
		)
	new_text = new_text.lstrip(" .")
	new_text = new_text.rstrip(" .")
	search_text = new_text
	set_search_filter_text(search_text)
	search_container.hide()
	container_content.show()


func _on_timer_search_debounce_timeout() -> void:
	search_container.set_keyword_search_text(search_text)


func _on_event_details_jump_in(parcel_position: Vector2i, realm: String) -> void:
	event_details.hide()
	Global.teleport_to(parcel_position, realm)


func _on_event_details_jump_in_world(realm: String) -> void:
	event_details.hide()
	Global.join_world(realm)


func _on_notification_clicked(notification_d: Dictionary) -> void:
	var notif_type = notification_d.get("type", "")

	if notif_type not in ["event_created", "events_starts_soon", "events_started"]:
		return

	var metadata = notification_d.get("metadata", {})

	var link = metadata.get("link", "")
	if link.is_empty():
		printerr("[Discover] Event notification missing link in metadata")
		_on_error_loading_notification()
		return

	var event_id = _extract_event_id_from_url(link)
	if event_id.is_empty():
		printerr("[Discover] Could not extract event ID from link: ", link)
		_on_error_loading_notification()
		return

	_async_handle_event_notification(event_id)


func _extract_event_id_from_url(url: String) -> String:
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

	on_event_pressed(event_data)


func _on_error_loading_notification() -> void:
	Global.close_navbar.emit()


func _on_report_loading_status(status: CarrouselGenerator.LoadingStatus, container) -> void:
	_generator_statuses[container] = status
	_update_global_messages()


func _get_active_carousels() -> Array:
	if search_text.is_empty():
		return [last_visited, places_featured, places_most_active, events, places_favorites]
	return [places_featured, places_most_active, events, places_favorites]


func _update_global_messages() -> void:
	%MessageError.hide()
	%MessageNoResultsFound.hide()

	var active := _get_active_carousels()
	var all_finished := true
	var all_error := true
	var any_has_items := false
	var any_with_results := false

	for container in active:
		if not _generator_statuses.has(container):
			all_finished = false
			all_error = false
			continue
		var s = _generator_statuses[container]
		if s == CarrouselGenerator.LoadingStatus.LOADING:
			all_finished = false
			all_error = false
			continue
		if s != CarrouselGenerator.LoadingStatus.ERROR:
			all_error = false
		if s == CarrouselGenerator.LoadingStatus.OK_WITH_RESULTS:
			any_with_results = true
		if container.has_items():
			any_has_items = true

	if any_with_results and not search_text.is_empty():
		if Global.get_config().add_search_history(search_text):
			Global.get_config().save_to_settings_file()

	if not all_finished:
		return

	if all_error:
		%MessageError.show()
	elif not any_has_items:
		%MessageNoResultsFound.show()


func _async_on_keyword_selected(keyword: SearchSuggestions.Keyword) -> void:
	var search_keyword := keyword.keyword
	if keyword.type == SearchSuggestions.KeywordType.COORDINATES:
		search_keyword = await PlacesHelper.async_get_name_from_coordinates(keyword.coordinates)
	search_bar.text = search_keyword
	search_text = search_keyword
	set_search_filter_text(search_keyword)
	search_container.hide()
	container_content.show()


func _on_button_back_to_explorer_pressed() -> void:
	if not search_bar.closed:
		search_bar.close_searchbar()
		search_text = ""
		set_search_filter_text("")
		search_container.hide()
		container_content.show()
		label_title.show()
		if not Global.get_explorer():
			button_back_to_explorer.hide()
		return
	if Global.get_explorer():
		Global.close_menu.emit()
		Global.set_orientation_landscape()
