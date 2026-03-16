class_name SearchSuggestions
extends Control

signal keyword_selected(keyword: Keyword)
signal should_show_container(show: bool)

enum KeywordType { HISTORY, POPULAR, CATEGORY, COORDINATES }

const FIELD_WEIGHTS: Dictionary = {
	"title": 1.0,
	"description": 0.4,
	"contact_name": 0.3,
	"world_name": 1.0,
	"world_id": 0.3,
}

var popular_keywords: Array[Keyword]
var coordinates_destinations: Dictionary[int, Dictionary]
# Trigram TF-IDF index: { "abc": { item_index: tfidf_score, ... }, ... }
var trigram_index: Dictionary = {}

@onready var search_sugestions := %SearchSugestions
@onready var margin_container_recent_searches: MarginContainer = %MarginContainer_RecentSearchs
@onready var button_clear_history: Button = %Button_ClearHistory


class Keyword:
	var keyword: String
	var type: KeywordType
	var coordinates: Vector2i
	var fields: Dictionary = {}  # { field_name: sanitized_text } for TF-IDF indexing

	func _init(keyword_param: String, type_param: KeywordType) -> void:
		keyword = keyword_param
		type = type_param


func _ready() -> void:
	margin_container_recent_searches.hide()
	button_clear_history.pressed.connect(_on_button_clear_history_pressed)
	async_get_popular_keywords()


func async_get_popular_keywords() -> void:
	const LIMIT := 100
	const TOTAL := 500

	var base_params := "&order_by=most_active&sdk=7"
	if Global.is_ios_or_emulating():
		base_params += "&tag=allowed_ios"

	var all_results: Array = []
	for page in range(float(TOTAL) / LIMIT):
		var url := (
			PlacesHelper.get_api_url() + "?offset=%d&limit=%d" % [page * LIMIT, LIMIT] + base_params
		)
		prints("SEARCH", url)
		var fetch_result: PlacesHelper.FetchResult = await PlacesHelper.async_fetch_places(url)
		match fetch_result.status:
			PlacesHelper.FetchResultStatus.ERROR:
				printerr("Error request places ", url, " ", fetch_result.promise_error.get_error())
				break
			PlacesHelper.FetchResultStatus.OK:
				all_results.append_array(fetch_result.result)

	popular_keywords.clear()
	for destination in all_results:
		prints("SEARCH", destination)
		# destination.id = url
		# destination.world_name

		# destination.title
		# destination.description
		# destination.contact_name

		var destination_name: String = NotificationUtils.sanitize_notification_text(
			trim_string(destination.title.to_lower())
		)

		if destination_name.length() >= 3:
			var kw := Keyword.new(destination_name, KeywordType.POPULAR)
			kw.fields["title"] = destination_name
			if destination.world:
				var world_name: String = NotificationUtils.sanitize_notification_text(
					trim_string(str(destination.get("world_name", "")).to_lower())
				)
				if world_name.length() >= 3:
					kw.fields["world_name"] = world_name
				var world_id: String = trim_string(str(destination.get("id", "")).to_lower())
				if world_id.length() >= 3:
					kw.fields["world_id"] = world_id
				if world_id.begins_with("tower"):
					print("Tower")
			else:
				var desc: String = NotificationUtils.sanitize_notification_text(
					trim_string(str(destination.get("description", "")).to_lower())
				)
				if desc.length() >= 3:
					kw.fields["description"] = desc
				var contact: String = NotificationUtils.sanitize_notification_text(
					trim_string(str(destination.get("contact_name", "")).to_lower())
				)
				if contact.length() >= 3:
					kw.fields["contact_name"] = contact
			popular_keywords.push_back(kw)

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
	build_trigram_index()


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
		_build_suggestions_ui(keywords_result, true, trimmed_search_text)
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
			keywords_result = search_popular_keywords(trimmed_search_text)
		should_show_container.emit(true)
		_build_suggestions_ui(keywords_result, false, trimmed_search_text)
		return

	should_show_container.emit(false)


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


func get_trigrams(text: String) -> Array[String]:
	var trigrams: Array[String] = []
	var n := text.length()
	if n < 3:
		return trigrams
	for i in range(n - 2):
		trigrams.append(text.substr(i, 3))
	return trigrams


func build_trigram_index() -> void:
	trigram_index.clear()
	var total_items := popular_keywords.size()
	if total_items == 0:
		return

	# Step 1: compute weighted TF per item across all fields, track document frequencies
	# weighted_tf[trigram] = sum over fields of (field_weight × tf_in_field)
	var tf_per_item: Array = []  # Array of Dictionary{ trigram: weighted_tf }
	var doc_freq: Dictionary = {}  # { trigram: count of items containing it }

	for i in range(total_items):
		var weighted_tf: Dictionary = {}
		var seen: Dictionary = {}  # trigrams seen in this item (for doc_freq, count once)

		for field_name: String in FIELD_WEIGHTS:
			var field_weight: float = FIELD_WEIGHTS[field_name]
			var text: String = popular_keywords[i].fields.get(field_name, "")
			if text.is_empty():
				continue
			var trigrams := get_trigrams(text)
			if trigrams.is_empty():
				continue
			var counts: Dictionary = {}
			for t in trigrams:
				counts[t] = counts.get(t, 0) + 1
			var total_trigrams := float(trigrams.size())
			for t: String in counts:
				var tf: float = float(counts[t]) / total_trigrams
				weighted_tf[t] = weighted_tf.get(t, 0.0) + field_weight * tf
				seen[t] = true

		for t: String in seen:
			doc_freq[t] = doc_freq.get(t, 0) + 1
		tf_per_item.append(weighted_tf)

	# Step 2: compute TF-IDF and build the inverted index
	# Uses smoothed IDF: log(1 + N/df) to avoid zero scores for frequent trigrams
	for i in range(total_items):
		for t: String in tf_per_item[i]:
			var idf := log(1.0 + float(total_items) / float(doc_freq[t]))
			var tfidf: float = tf_per_item[i][t] * idf
			if not trigram_index.has(t):
				trigram_index[t] = {}
			trigram_index[t][i] = tfidf


func search_popular_keywords(search_text: String) -> Array[Keyword]:
	var query_trigrams := get_trigrams(search_text)
	if query_trigrams.is_empty():
		return []

	# Accumulate TF-IDF scores across all query trigrams
	var item_scores: Dictionary = {}
	for t in query_trigrams:
		if trigram_index.has(t):
			for item_idx: int in trigram_index[t]:
				item_scores[item_idx] = item_scores.get(item_idx, 0.0) + trigram_index[t][item_idx]

	if item_scores.is_empty():
		return []

	# Sort item indices by score descending
	var sorted_indices: Array = item_scores.keys()
	sorted_indices.sort_custom(func(a: int, b: int) -> bool: return item_scores[a] > item_scores[b])

	var result: Array[Keyword] = []
	for idx: int in sorted_indices:
		result.append(popular_keywords[idx])
	return result


func trim_string(text: String) -> String:
	const STRIP_CHARS = ".*!?¡¿-_[]<>'\"%&\\/,.;:"
	text = text.lstrip(STRIP_CHARS)
	text = text.rstrip(STRIP_CHARS)
	text = text.strip_edges()
	return text
