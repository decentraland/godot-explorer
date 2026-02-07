class_name SearchSuggestions
extends Control

signal keyword_selected(keyword: Keyword)
signal should_show_container(show: bool)

enum KeywordType { HISTORY, POPULAR, CATEGORY, COORDINATES }

var popular_keywords: Array[Keyword]
var coordinates_destinations: Dictionary[int, Dictionary]

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
	async_get_popular_keywords()


func async_get_popular_keywords() -> void:
	var url := PlacesHelper.get_api_url()
	url += "?offset=0&limit=100"
	url += "&order_by=most_active"
	url += "&sdk=7"
	if Global.is_ios_or_emulating():
		url += "&tag=allowed_ios"

	var fetch_result: PlacesHelper.FetchResult = await PlacesHelper.async_fetch_places(url)
	match fetch_result.status:
		PlacesHelper.FetchResultStatus.ERROR:
			printerr("Error request places ", url, " ", fetch_result.premise_error.get_error())
			return
		PlacesHelper.FetchResultStatus.OK:
			pass

	for destination in fetch_result.result:
		if destination.world:
			continue
		var destination_name: String = NotificationUtils.sanitize_notification_text(
			trim_string(destination.title.to_lower())
		)

		if destination_name.length() >= 3:
			popular_keywords.push_back(Keyword.new(destination_name, KeywordType.POPULAR))

		for p: String in destination.positions:
			var p_split := p.split(",")
			var px := p_split[0].to_int()
			var py := p_split[1].to_int()
			if not coordinates_destinations.has(px):
				coordinates_destinations[px] = {}
			coordinates_destinations[px][py] = {
				"title": destination.title,
				"id": destination.id,
				"first_position": destination.positions[0]
			}


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

	if text_len < 2:
		var count_history := 0
		for k in Global.get_config().search_history:
			if count_history >= 4:
				break
			var hist_keyword := Keyword.new(k, KeywordType.HISTORY)
			if (
				trimmed_search_text.is_empty()
				or hist_keyword.keyword.contains(trimmed_search_text)
				or hist_keyword.keyword.similarity(trimmed_search_text) > 0.2
			):
				keywords_result.push_back(hist_keyword)
				count_history += 1
		if keywords_result.is_empty():
			should_show_container.emit(false)
			return
		should_show_container.emit(true)
		_build_suggestions_ui(keywords_result, true)
		return

	if text_len >= 3:
		if is_coordinates:
			if coordinates_destinations.has(coordinates.x):
				if coordinates_destinations[coordinates.x].has(coordinates.y):
					var title: String = coordinates_destinations[coordinates.x][coordinates.y].title
					var coordinate_keyword := Keyword.new(title, KeywordType.COORDINATES)
					coordinate_keyword.coordinates = Vector2i(coordinates.x, coordinates.y)
					keywords_result.push_back(coordinate_keyword)
				else:
					var shown_destinations: Array[String]
					for d in coordinates_destinations[coordinates.x]:
						var dd: Dictionary = coordinates_destinations[coordinates.x][d]
						if shown_destinations.has(dd.id):
							continue
						var title: String = trim_string(dd.title)
						title = NotificationUtils.sanitize_notification_text(title, true, false)
						var p_split: Array = dd.first_position.split(",")
						var px: int = p_split[0].to_int()
						var py: int = p_split[1].to_int()
						var coordinate_keyword := Keyword.new(title, KeywordType.COORDINATES)
						coordinate_keyword.coordinates = Vector2i(px, py)
						keywords_result.push_back(coordinate_keyword)
						shown_destinations.append(dd.id)
		else:
			for k in popular_keywords:
				if (
					k.keyword.contains(trimmed_search_text)
					or k.keyword.similarity(trimmed_search_text) > 0.2
				):
					keywords_result.push_back(k)
		should_show_container.emit(true)
		_build_suggestions_ui(keywords_result, false)
		return

	should_show_container.emit(false)


func _build_suggestions_ui(keywords_result: Array[Keyword], show_recent_label: bool) -> void:
	margin_container_recent_searches.visible = show_recent_label
	for c in search_sugestions.get_children():
		c.queue_free()

	const SEARCH = preload("res://src/ui/components/discover/icons/search-outlined.svg")
	const CLOCK = preload("res://src/ui/components/discover/icons/clock.svg")
	const MAP = preload("res://src/ui/components/discover/icons/world-outlined.svg")
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
