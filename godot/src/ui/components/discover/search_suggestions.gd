class_name SearchSuggestions
extends Control

signal keyword_selected(keyword: String)

enum KeywordType { HISTORY, POPULAR, CATEGORY, COORDINATES }

var popular_keywords: Array[Keyword]

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
	url += "?offset=0&limit=10"
	url += "&order_by=most_active"

	var fetch_result: PlacesHelper.FetchResult = await PlacesHelper.async_fetch_places(url)
	match fetch_result.status:
		PlacesHelper.FetchResultStatus.ERROR:
			printerr("Error request places ", url, " ", fetch_result.premise_error.get_error())
			return
		PlacesHelper.FetchResultStatus.OK:
			pass

	for destination in fetch_result.result:
		var distionation_name: String = destination.title.to_lower()
		if distionation_name.length() >= 3:
			popular_keywords.push_back(Keyword.new(distionation_name, KeywordType.POPULAR))

		var destination_textsearch: String = destination.textsearch.to_lower()
		var regex = RegEx.new()
		regex.compile("\'(?<textsearch>.+?)\'")
		var regex_match := regex.search_all(destination_textsearch)
		for m in regex_match:
			var textsearch := m.strings[1]
			if textsearch.length() < 3:
				continue
			popular_keywords.push_back(Keyword.new(textsearch, KeywordType.POPULAR))


func set_keyword_search_text(_search_text: String) -> void:
	if _search_text.length() < 3:
		_search_text = ""

	var keywords_available: Array[Keyword] = []
	var keywords_result: Array[Keyword] = []

	var coordinates := {}
	var is_coordinates := PlacesHelper.parse_coordinates(_search_text, coordinates)

	if is_coordinates:
		var coordinates_string := await PlacesHelper.async_replace_get_name_from_coordinates(
			Vector2i(coordinates.x, coordinates.y)
		)

		if coordinates_string == "":
			coordinates_string = "%d,%d" % [coordinates.x, coordinates.y]
		var coordinate_keyword := Keyword.new(coordinates_string, KeywordType.COORDINATES)
		coordinate_keyword.coordinates = Vector2i(coordinates.x, coordinates.y)
		keywords_result.push_back(coordinate_keyword)

	for k in Global.get_config().search_history:
		keywords_available.append(Keyword.new(k, KeywordType.HISTORY))

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

	for k in keywords_result:
		var keyword := Button.new()
		keyword.pressed.connect(_on_keyword_selected.bind(k))
		keyword.add_theme_color_override("font_color", Color.BLACK)
		keyword.add_theme_font_size_override("font_size", 30)

		keyword.icon = CLOCK if k.type == KeywordType.HISTORY else SEARCH
		keyword.text = k.keyword
		search_sugestions.add_child(keyword)


func _on_keyword_selected(keyword: Keyword) -> void:
	keyword_selected.emit(keyword)
