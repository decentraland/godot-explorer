class_name SearchSuggestions
extends Control

signal keyword_selected(keyword: Keyword)
signal should_show_container(show: bool)

enum KeywordType { HISTORY, POPULAR, CATEGORY, COORDINATES, EVENT }

const SEARCH_DEBOUNCE_SEC := 0.4
const SEARCH_LIMIT := 15

var _pending_search_text: String = ""
var _pending_coordinates: Vector2i = Vector2i.ZERO
var _pending_is_coordinates: bool = false
var _has_api_results: bool = false
var _search_timer: Timer

@onready var search_sugestions := %SearchSugestions
@onready var margin_container_recent_searches: MarginContainer = %MarginContainer_RecentSearchs
@onready var button_clear_history: Button = %Button_ClearHistory


class Keyword:
	var keyword: String
	var type: KeywordType
	var coordinates: Vector2i

	func _init(keyword_param: String, type_param: KeywordType) -> void:
		keyword = keyword_param
		type = type_param


func _ready() -> void:
	margin_container_recent_searches.hide()
	button_clear_history.pressed.connect(_on_button_clear_history_pressed)
	_search_timer = Timer.new()
	_search_timer.wait_time = SEARCH_DEBOUNCE_SEC
	_search_timer.one_shot = true
	_search_timer.timeout.connect(_on_search_timer_timeout)
	add_child(_search_timer)


func set_keyword_search_text(_search_text: String) -> void:
	var keywords_result: Array[Keyword] = []
	var trimmed_search_text := trim_string(_search_text)
	var text_len := trimmed_search_text.length()

	var coordinates := {}
	var is_coordinates := false
	if text_len >= 3:
		is_coordinates = PlacesHelper.parse_coordinates(_search_text, coordinates)
		if not is_coordinates and trimmed_search_text.is_valid_int():
			is_coordinates = true
			coordinates.x = trimmed_search_text.to_int()
			coordinates.y = int(INF)

	if text_len <= 2:
		_search_timer.stop()
		_pending_search_text = ""
		_pending_is_coordinates = false
		# Keep API results visible while the user edits; only show recent searches
		# when no API results are currently displayed.
		if not _has_api_results:
			var count_history := 0
			for k in Global.get_config().search_history:
				if count_history >= 4:
					break
				keywords_result.push_back(Keyword.new(k, KeywordType.HISTORY))
				count_history += 1
			should_show_container.emit(true)
			_build_suggestions_ui(
				keywords_result, not keywords_result.is_empty(), trimmed_search_text
			)
		return

	if text_len >= 3:
		if is_coordinates:
			_pending_search_text = trimmed_search_text
			_pending_coordinates = Vector2i(coordinates.x, coordinates.y)
			_pending_is_coordinates = true
		else:
			_pending_search_text = trimmed_search_text
			_pending_is_coordinates = false
		should_show_container.emit(true)
		_search_timer.start()
		return


func stop_suggestions() -> void:
	_search_timer.stop()
	_pending_search_text = ""
	_pending_is_coordinates = false
	_has_api_results = false


func _on_search_timer_timeout() -> void:
	if _pending_is_coordinates:
		async_search_coordinates(_pending_search_text, _pending_coordinates)
	else:
		async_search_places(_pending_search_text)


func async_search_coordinates(search_text: String, coordinates: Vector2i) -> void:
	var name := await PlacesHelper.async_get_name_from_coordinates(coordinates)
	if search_text != _pending_search_text:
		return
	var keywords_result: Array[Keyword] = []
	if not name.is_empty():
		var kw := Keyword.new(name, KeywordType.COORDINATES)
		kw.coordinates = coordinates
		keywords_result.push_back(kw)
	_has_api_results = true
	should_show_container.emit(true)
	_build_suggestions_ui(keywords_result, false, search_text)


func async_search_places(search_text: String) -> void:
	var encoded := search_text.uri_encode()
	var ios_tag := "&tag=allowed_ios" if Global.is_ios_or_emulating() else ""

	var places_url := (
		PlacesHelper.get_api_url()
		+ "?search=%s&limit=%d&sdk=7%s" % [encoded, SEARCH_LIMIT, ios_tag]
	)
	var events_url := DclUrls.mobile_events_api() + "/?sdk=7&search=%s%s" % [encoded, ios_tag]

	var places_result: PlacesHelper.FetchResult = await PlacesHelper.async_fetch_places(places_url)
	var events_response = await Global.async_signed_fetch(events_url, HTTPClient.METHOD_GET, "")

	# Discard if the user has already typed something different
	if search_text != _pending_search_text:
		return

	var places_keywords: Array[Keyword] = []
	var events_keywords: Array[Keyword] = []

	if places_result.status == PlacesHelper.FetchResultStatus.ERROR:
		printerr(
			"Error searching places ", places_url, " ", places_result.promise_error.get_error()
		)
	else:
		for destination in places_result.result:
			var destination_name: String = NotificationUtils.sanitize_notification_text(
				trim_string(destination.title.to_lower())
			)
			if destination_name.length() >= 3:
				places_keywords.push_back(Keyword.new(destination_name, KeywordType.POPULAR))

	if events_response is PromiseError:
		printerr("Error searching events ", events_url, " ", events_response.get_error())
	else:
		var json: Dictionary = events_response.get_string_response_as_json()
		if json.has("data"):
			for event_data in json.data:
				if not event_data.get("approved", false):
					continue
				var event_name: String = NotificationUtils.sanitize_notification_text(
					trim_string(str(event_data.get("name", "")).to_lower())
				)
				if event_name.length() >= 3:
					events_keywords.push_back(Keyword.new(event_name, KeywordType.EVENT))

	# Interleave places and events: place, event, place, event, ...
	var keywords_result: Array[Keyword] = []
	var max_len := maxi(places_keywords.size(), events_keywords.size())
	for i in range(max_len):
		if i < places_keywords.size():
			keywords_result.push_back(places_keywords[i])
		if i < events_keywords.size():
			keywords_result.push_back(events_keywords[i])

	_has_api_results = true
	should_show_container.emit(true)
	_build_suggestions_ui(keywords_result, false, search_text)


func _build_suggestions_ui(
	keywords_result: Array[Keyword], show_recent_label: bool, search_query: String = ""
) -> void:
	margin_container_recent_searches.visible = show_recent_label
	(
		Global
		. metrics
		. track_screen_viewed(
			"SEARCH_SHOW_SUGGESTIONS",
			JSON.stringify(
				{"search_query": search_query, "suggestions_count": keywords_result.size()}
			),
		)
	)
	for c in search_sugestions.get_children():
		c.queue_free()

	const SEARCH = preload("res://src/ui/components/discover/icons/search-outlined.svg")
	const CLOCK = preload("res://src/ui/components/discover/icons/clock.svg")
	const MAP = preload("res://src/ui/components/discover/icons/world-outlined.svg")
	const CALENDAR = preload("res://src/ui/components/discover/icons/calendar-outlined.svg")
	const KEYWORD = preload("res://src/ui/components/discover/search_keyword.tscn")

	for k in keywords_result:
		var keyword := KEYWORD.instantiate()
		keyword.pressed.connect(_on_keyword_selected.bind(k))
		match k.type:
			KeywordType.HISTORY:
				keyword.icon = CLOCK
			KeywordType.POPULAR:
				keyword.icon = SEARCH
			KeywordType.COORDINATES:
				keyword.icon = MAP
			KeywordType.EVENT:
				keyword.icon = CALENDAR
		keyword.text = k.keyword.capitalize()
		search_sugestions.add_child(keyword)


func _on_keyword_selected(keyword: Keyword) -> void:
	keyword_selected.emit(keyword)


func _on_button_clear_history_pressed() -> void:
	var empty_history: Array[String] = []
	Global.get_config().search_history = empty_history
	Global.get_config().save_to_settings_file()
	set_keyword_search_text("")


func trim_string(text: String) -> String:
	const STRIP_CHARS = ".*!?¡¿-_[]<>'\"%&\\/,.;:"
	text = text.lstrip(STRIP_CHARS)
	text = text.rstrip(STRIP_CHARS)
	text = text.strip_edges()
	return text
