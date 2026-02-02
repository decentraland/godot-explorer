class_name SearchSuggestions
extends Control

signal keyword_selected(keyword: String)

enum KeywordType { HISTORY, POPULAR, CATEGORY, COORDINATES }

var popular_keywords: Array[Keyword]
var coordinates_destinations: Dictionary[int, Dictionary]

@onready var search_sugestions := %SearchSugestions


class Keyword:
	var keyword: String
	var type: KeywordType
	var coordinates: Vector2i

	func _init(keyword_param: String, type_param: KeywordType) -> void:
		keyword = keyword_param
		type = type_param


func _ready() -> void:
	async_get_popular_keywords()


func async_get_popular_keywords() -> void:
	var url := PlacesHelper.get_api_url()
	url += "?offset=0&limit=100"
	url += "&order_by=most_active"

	var fetch_result: PlacesHelper.FetchResult = await PlacesHelper.async_fetch_places(url)
	match fetch_result.status:
		PlacesHelper.FetchResultStatus.ERROR:
			printerr("Error request places ", url, " ", fetch_result.premise_error.get_error())
			return
		PlacesHelper.FetchResultStatus.OK:
			pass

	for destination in fetch_result.result:
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
			coordinates_destinations[px][py] = destination

		#var destination_textsearch: String = destination.textsearch.to_lower()
		#var regex = RegEx.new()
		#regex.compile("\'(?<textsearch>.+?)\'")
		#var regex_match := regex.search_all(destination_textsearch)
		#for m in regex_match:
		#var textsearch := m.strings[1]
		#if textsearch.length() < 3:
		#continue
		#popular_keywords.push_back(Keyword.new(textsearch, KeywordType.POPULAR))


func set_keyword_search_text(_search_text: String) -> void:
	#if _search_text.length() < 3:
	#	_search_text = ""

	var keywords_available: Array[Keyword] = []
	var keywords_result: Array[Keyword] = []

	var coordinates := {}
	var is_coordinates := PlacesHelper.parse_coordinates(_search_text, coordinates)
	var trimmed_search_text := trim_string(_search_text)
	if trimmed_search_text.is_valid_int():
		is_coordinates = true
		coordinates.x = trimmed_search_text.to_int()
		coordinates.y = INF
	if is_coordinates:
		if coordinates_destinations.has(coordinates.x):
			if coordinates_destinations[coordinates.x].has(coordinates.y):
				var title: String = coordinates_destinations[coordinates.x][coordinates.y].title
				var coordinate_keyword := Keyword.new(title, KeywordType.COORDINATES)
				coordinate_keyword.coordinates = Vector2i(coordinates.x, coordinates.y)
				keywords_result.push_back(coordinate_keyword)
			elif coordinates_destinations[coordinates.x].size() > 0:
				# If only the first coordinate was entered
				var shown_destinations: Array[String]
				for d in coordinates_destinations[coordinates.x]:
					var dd: Dictionary = coordinates_destinations[coordinates.x][d]
					# Don't display the same destination twice
					if shown_destinations.has(dd.id):
						continue
					var title: String = trim_string(dd.title)
					title = NotificationUtils.sanitize_notification_text(title, true, false)
					var p_split: Array = dd.positions[0].split(",")
					var px: int = p_split[0].to_int()
					var py: int = p_split[1].to_int()
					var coordinate_keyword := Keyword.new(title, KeywordType.COORDINATES)
					coordinate_keyword.coordinates = Vector2i(px, py)
					keywords_result.push_back(coordinate_keyword)
					shown_destinations.append(dd.id)

	for k in Global.get_config().search_history:
		keywords_available.append(Keyword.new(k, KeywordType.HISTORY))

	#var coordinates := {}
	#var is_coordinates := PlacesHelper.parse_coordinates(_search_text, coordinates)
#
	#if is_coordinates:
	#var coordinates_string := await PlacesHelper.async_get_name_from_coordinates(
	#Vector2i(coordinates.x, coordinates.y)
	#)
#
	#if coordinates_string == "":
	#coordinates_string = "%d,%d" % [coordinates.x, coordinates.y]
	#var coordinate_keyword := Keyword.new(coordinates_string, KeywordType.COORDINATES)
	#coordinate_keyword.coordinates = Vector2i(coordinates.x, coordinates.y)
	#keywords_result.push_back(coordinate_keyword)
#

	#var response = await PlacesHelper.async_get_by_names(_search_text)
	#if response:
	#if response is PromiseError:
	#printerr("Error request places ", _search_text, " ", response.get_error())
	#var json: Dictionary = response.get_string_response_as_json()
	#if not json.data.is_empty():
	#for s in json.data:
	#keywords_available.append(Keyword.new(s.title, KeywordType.POPULAR))

	for k in popular_keywords:
		keywords_available.append(k)

	var count_history := 0
	for k in keywords_available:
		if (
			_search_text == ""
			or k.keyword.contains(_search_text)
			or k.keyword.similarity(_search_text) > 0.2
		):
			if k.type == KeywordType.HISTORY:
				if count_history >= 4:
					continue
				else:
					count_history += 1
			keywords_result.push_back(k)

	for c in search_sugestions.get_children():
		c.queue_free()

	const SEARCH = preload("res://src/ui/components/debug_panel/icons/Search.svg")
	const CLOCK = preload("res://assets/ui/clock.svg")
	const MAP = preload("res://assets/maphud.svg")

	for k in keywords_result:
		var keyword := Button.new()
		keyword.alignment = HORIZONTAL_ALIGNMENT_LEFT
		keyword.mouse_filter = Control.MOUSE_FILTER_PASS
		keyword.flat = true
		keyword.expand_icon = true
		keyword.add_theme_constant_override("icon_max_width", 64)
		keyword.pressed.connect(_on_keyword_selected.bind(k))
		keyword.add_theme_color_override("font_color", Color.WHITE)
		keyword.add_theme_font_size_override("font_size", 30)

		match k.type:
			KeywordType.HISTORY:
				keyword.icon = CLOCK
			KeywordType.POPULAR:
				keyword.icon = SEARCH
			KeywordType.COORDINATES:
				keyword.icon = MAP
		keyword.text = k.keyword
		search_sugestions.add_child(keyword)


func _on_keyword_selected(keyword: Keyword) -> void:
	keyword_selected.emit(keyword)


func trim_string(text: String) -> String:
	const STRIP_CHARS = " .*!?¡¿-_[]<>'\"%&\\/,.;:"
	text = text.lstrip(STRIP_CHARS)
	text = text.rstrip(STRIP_CHARS)
	return text
