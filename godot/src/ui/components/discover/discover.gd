class_name Discover
extends Control

var search_text: String = ""
var _generator_statuses: Dictionary = {}

@onready var jump_in: SidePanelWrapper = %JumpIn
@onready var event_details: SidePanelWrapper = %EventDetails

@onready var search_bar: SearchBar = %SearchBar

@onready var friends_online: VBoxContainer = %FriendsOnline
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
@onready var friend_jump_in: SidePanelWrapper = %FriendJumpIn
#@onready var discover_content: VBoxContainer = %DiscoverContent

static var _low_spec_warning_shown: bool = false


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
	friend_jump_in.jump_in.connect(_on_friend_jump_in)
	friend_jump_in.jump_in_world.connect(_on_friend_jump_in_world)
	friend_jump_in.hide()

	Global.notification_clicked.connect(_on_notification_clicked)

	search_container.hide()
	search_container.keyword_selected.connect(_async_on_keyword_selected)
	search_container.should_show_container.connect(_on_should_show_suggestions_container)
	search_bar.cleared.connect(_on_search_bar_cleared)
	container_content.show()

	friends_online.generator.report_loading_status.connect(
		_on_report_loading_status.bind(friends_online)
	)
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


func on_friend_pressed(data):
	friend_jump_in.set_data(data)
	friend_jump_in.open_panel()
	# Set friend info on the instantiated panel
	_async_set_friend_info_on_panel(data)


func _async_set_friend_info_on_panel(data: Dictionary) -> void:
	# Wait a frame for the panel to be instantiated
	await get_tree().process_frame
	var panel: PlaceItem = friend_jump_in.portrait_panel
	if not panel:
		panel = friend_jump_in.landscape_panel
	if not panel:
		return

	var friend_name_label = panel.get_node_or_null("%Label_FriendName")
	if friend_name_label:
		friend_name_label.text = data.get("_friend_name", "")

	var has_claimed_name: bool = data.get("_friend_has_claimed_name", false)

	var friend_tag_label = panel.get_node_or_null("%Label_FriendTag")
	if friend_tag_label:
		if has_claimed_name:
			friend_tag_label.hide()
		else:
			var address: String = data.get("_friend_address", "")
			if not address.is_empty():
				friend_tag_label.text = "#" + address.substr(2, 4)
			else:
				friend_tag_label.text = ""
			friend_tag_label.show()

	var checkmark = panel.get_node_or_null("%TextureRect_ClaimedCheckmark")
	if checkmark:
		checkmark.visible = has_claimed_name

	var profile_pic = panel.get_node_or_null("%ProfilePicture")
	if profile_pic:
		var social_data = SocialItemData.new()
		social_data.name = data.get("_friend_name", "")
		social_data.address = data.get("_friend_address", "")
		social_data.profile_picture_url = data.get("_friend_profile_picture_url", "")
		social_data.has_claimed_name = data.get("_friend_has_claimed_name", false)
		profile_pic.async_update_profile_picture(social_data)


func _on_friend_jump_in(parcel_position: Vector2i, realm: String):
	friend_jump_in.hide()
	Global.async_teleport_to(parcel_position, realm)


func _on_friend_jump_in_world(realm: String):
	friend_jump_in.hide()
	Global.async_join_world(realm)


func on_event_pressed(data):
	if data is String:
		_async_handle_event_notification(data)
		return
	event_details.set_data(data)
	event_details.open_panel()


func async_open_event_by_id(event_id: String) -> void:
	_async_handle_event_notification(event_id)


func async_open_place_by_id(place_id: String) -> void:
	var response = await PlacesHelper.async_get_place_by_id(place_id)
	if response is PromiseError:
		printerr("[Discover] Failed to fetch place data: ", response.get_error())
		return
	var json: Dictionary = response.get_string_response_as_json()
	var place_data: Dictionary = json.get("data", json)
	if place_data.is_empty():
		printerr("[Discover] Empty place data for id: ", place_id)
		return
	on_item_pressed(place_data)


func _on_jump_in_jump_in(parcel_position: Vector2i, realm: String):
	jump_in.hide()
	Global.async_teleport_to(parcel_position, realm)


func _on_jump_in_world(realm: String):
	jump_in.hide()
	Global.async_join_world(realm)


func _get_ui_location() -> String:
	return "in_game" if Global.get_explorer() else "pre_game"


func _on_visibility_changed():
	if is_node_ready() and is_inside_tree() and is_visible_in_tree():
		last_visited.generator.async_request_last_places(0, 10)
		friends_online.generator.on_request(0, 10)
		Global.set_orientation_portrait()
		Global.metrics.track_screen_viewed(
			"DISCOVER", JSON.stringify({"location": _get_ui_location()})
		)
		_show_low_spec_warning_if_needed()
		if Global.get_explorer():
			if button_back_to_explorer:
				button_back_to_explorer.show()


func _show_low_spec_warning_if_needed():
	# Skip if already shown this session
	if _low_spec_warning_shown:
		return

	var deeplink_warning = Global.deep_link_obj and Global.deep_link_obj.low_spec_warning
	var is_ftue = not Global.get_config().low_spec_warning_shown
	var is_low_spec = DclIosPlugin.is_available() and DclIosPlugin.is_low_spec_iphone()

	# Show if: deep link forces it OR (FTUE and low-spec device)
	if not deeplink_warning and not (is_ftue and is_low_spec):
		return

	_low_spec_warning_shown = true

	# Persist FTUE flag (not for deep link bypass)
	if is_ftue and is_low_spec:
		Global.get_config().low_spec_warning_shown = true
		Global.get_config().save_to_settings_file()

	Global.metrics.track_screen_viewed("MINSPEC_PROMPT", "")
	Global.modal_manager.async_show_low_spec_iphone_modal()


func _on_search_bar_opened() -> void:
	button_back_to_explorer.show()
	label_title.hide()
	search_container.show()
	container_content.hide()
	search_container.set_keyword_search_text("")
	Global.metrics.track_click_button("SEARCH_SELECT_INPUT", "SEARCH_CLICK", "")


func _on_search_bar_cleared() -> void:
	search_text = ""
	set_search_filter_text("")
	search_container.stop_suggestions()
	search_container.show()
	search_container.set_keyword_search_text("")
	Global.metrics.track_click_button("SEARCH_ERASE", "SEARCH_CLICK", "")


func set_search_filter_text(new_text: String) -> void:
	_generator_statuses.clear()
	if new_text.is_empty():
		friends_online.visible = friends_online.has_items()
		last_visited.visible = last_visited.has_items()
		places_featured.show()
		places_favorites.show()
		places_my_places.visible = places_my_places.has_items()
		places_most_active.title = "Most Actives"
	else:
		friends_online.hide()
		last_visited.hide()
		places_featured.hide()
		places_favorites.hide()
		places_my_places.hide()
		places_most_active.title = "Scenes"
	places_most_active.set_search_param(new_text)
	events.set_search_param(new_text)
	_scroll_all_carousels_to_start()


func _scroll_all_carousels_to_start() -> void:
	container_content.reset_position()
	for carousel in [
		friends_online,
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
	search_container.stop_suggestions()
	search_container.hide()
	container_content.show()


func _on_event_details_jump_in(parcel_position: Vector2i, realm: String) -> void:
	event_details.hide()
	Global.async_teleport_to(parcel_position, realm)


func _on_event_details_jump_in_world(realm: String) -> void:
	event_details.hide()
	Global.async_join_world(realm)


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
		return [
			friends_online,
			last_visited,
			places_featured,
			places_most_active,
			events,
			places_favorites
		]
	return [places_most_active, events]


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

	if not search_text.is_empty():
		var results_count := 0
		var carousels_count := 0
		for container in active:
			if container.has_items():
				carousels_count += 1
				results_count += container.item_container.get_children().size()
		(
			Global
			. metrics
			. track_screen_viewed(
				"SEARCH_SHOW_RESULTS",
				(
					JSON
					. stringify(
						{
							"search_query": search_text,
							"results_count": results_count,
							"carousels_count": carousels_count,
						}
					)
				),
			)
		)
	else:
		# TODO: further define the carousel data format for the DISCOVER screen event
		var carousels_data := _collect_carousel_data()
		if not carousels_data.is_empty():
			(
				Global
				. metrics
				. track_screen_viewed(
					"DISCOVER",
					JSON.stringify({"location": _get_ui_location(), "carousels": carousels_data}),
				)
			)

	if all_error:
		%MessageError.show()
	elif not any_has_items:
		%MessageNoResultsFound.show()


func _collect_carousel_data() -> Dictionary:
	var result := {}
	var carousel_map := {
		"featured": places_featured,
		"most_active": places_most_active,
		"events": events,
		"last_visited": last_visited,
	}
	for key in carousel_map:
		var carousel = carousel_map[key]
		if not carousel.visible:
			continue
		var items := []
		var idx := 0
		for child in carousel.item_container.get_children():
			if child is PlaceItem:
				(
					items
					. append(
						{
							"id": child._data.get("id", ""),
							"type": "world" if child._data.get("world", false) else "scene",
							"position": idx,
						}
					)
				)
				idx += 1
		if not items.is_empty():
			result[key] = items
	return result


func _async_on_keyword_selected(keyword: SearchSuggestions.Keyword) -> void:
	(
		Global
		. metrics
		. track_click_button(
			"SEARCH_TAP_SUGGESTION",
			"SEARCH_CLICK",
			JSON.stringify({"search_query": search_text, "suggestion_text": keyword.keyword}),
		)
	)
	var search_keyword := keyword.keyword
	if keyword.type == SearchSuggestions.KeywordType.COORDINATES:
		search_keyword = await PlacesHelper.async_get_name_from_coordinates(keyword.coordinates)
	search_bar.text = search_keyword
	search_text = search_keyword
	set_search_filter_text(search_keyword)
	search_container.stop_suggestions()
	search_container.hide()
	container_content.show()


func _on_button_back_to_explorer_pressed() -> void:
	if not search_bar.closed:
		Global.metrics.track_click_button("SEARCH_GOBACK", "SEARCH_CLICK", "")
		search_bar.close_searchbar()
		search_text = ""
		set_search_filter_text("")
		search_container.stop_suggestions()
		search_container.hide()
		container_content.show()
		label_title.show()
		if not Global.get_explorer():
			button_back_to_explorer.hide()
		return
	if Global.get_explorer():
		if Global.modal_manager.ban_pre_check_active:
			Global.modal_manager.async_show_ban_pre_check_modal()
			return
		Global.close_menu.emit()
		Global.set_orientation_landscape()
