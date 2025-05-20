extends Control

@export var map: MapComponent
@onready var searchbar: PanelContainer = %Searchbar
@onready var archipelago_button: Button = %ArchipelagoButton
@onready var h_box_container_filters: HBoxContainer = %HBoxContainer_Filters
@onready var no_results: VBoxContainer = %NoResults
@onready var cards: BoxContainer = %Cards
@onready var cards_scroll: ScrollContainer = %CardsScroll
@onready var portrait_container: MarginContainer = %PortraitContainer
@onready var landscape_container: MarginContainer = %LandscapeContainer
@onready var search_results: Control = %SearchResults
@onready var portrait_panel_container: PanelContainer = %PortraitPanelContainer
@onready var landscape_panel_container: PanelContainer = %LandscapePanelContainer
@onready var search_results_container: Control = %SearchResultsContainer

const DISCOVER_CARROUSEL_ITEM = preload("res://src/ui/components/discover/carrousel/discover_carrousel_item.tscn")
const PLACE_CATEGORY_FILTER_BUTTON = preload("res://src/ui/components/map_satellite/place_category_filter_button.tscn")

var active_filter: int = -1
var filtered_places: Array = []
var is_closed: bool = true
var ignore_button_signals := false
var closed_position: Vector2

func _ready() -> void:
	get_window().size_changed.connect(self._on_size_changed)
	_on_size_changed()
	
	var group := ButtonGroup.new()
	group.allow_unpress = true
	
	for i in range(13):
		var btn: PlaceFilterButton = PLACE_CATEGORY_FILTER_BUTTON.instantiate()
		btn.button_group = group
		btn.toggle_mode = true
		btn.filter_type = i
		btn.connect("filter_toggled", Callable(self, "_on_filter_button_toggled"))
		h_box_container_filters.add_child(btn)
	var poi_places = await async_load_category(13)
	map.create_pins(13, poi_places, 'poi_pins')
	map.get_poi_ids(poi_places)
	var live_places = await async_load_category(14)
	map.create_pins(14, live_places, 'live_pins')
	map.async_draw_archipelagos()
	searchbar.clean_searchbar.connect(_close_from_searchbar)
	searchbar.submited_text.connect(_submitted_text_from_searchbar)
	searchbar.reset()

func _close_from_searchbar():
	_close_sidebar()
	_clean_list()
	
func _submitted_text_from_searchbar(text:String):
	var places_to_show = 0
	_clean_list()
	filtered_places = await async_load_text_search(text)
	map.create_pins(0, filtered_places, 'pins')
	for i in range(filtered_places.size()):
			var place = filtered_places[i]
			if place.title != "Empty":
				create_place_card(place)
				places_to_show = places_to_show + 1
	if places_to_show == 0:
		no_results.show()
		cards_scroll.hide()
	else:
		no_results.hide()
		cards_scroll.show()
	_open_sidebar()

func _on_filter_button_toggled(pressed: bool, type: int):
	var places_to_show = 0
	_clean_list()
	map.clear_pins()
	if not pressed:
		if ignore_button_signals:
			return
		filtered_places = []
		searchbar.reset()
	else:
		active_filter = type
		filtered_places = await async_load_category(active_filter)
		for i in range(filtered_places.size()):
			var place = filtered_places[i]
			if place.title == "Empty":
				continue
			create_place_card(place)
			places_to_show = places_to_show + 1
		if places_to_show == 0:
			no_results.show()
			cards_scroll.hide()
		else:
			no_results.hide()
			cards_scroll.show()
		_open_sidebar()
		
		searchbar.filter_type = type
		searchbar.update_filtered_category()
		map.create_pins(type, filtered_places, 'pins')

func _open_sidebar()->void:
	is_closed = false
	var tween = create_tween()
	tween.tween_property(search_results_container, "position", Vector2.ZERO, 0.4).set_trans(Tween.TRANS_LINEAR).set_ease(Tween.EASE_OUT)
	if Global.is_orientation_portrait():
		portrait_panel_container.show()
		landscape_panel_container.hide()
	else:
		landscape_panel_container.show()
		portrait_panel_container.hide()

func _close_sidebar()->void:
	is_closed = true
	cards_scroll.scroll_horizontal = 0
	cards_scroll.scroll_vertical = 0
	filtered_places = []
	for child in h_box_container_filters.get_children():
		if child is Button and child.toggle_mode and child.filter_type == active_filter:
			ignore_button_signals = true
			child.button_pressed = false
			ignore_button_signals = false
	portrait_panel_container.hide()
	landscape_panel_container.hide()
	search_results_container.position = closed_position

func _clean_list()->void:
	for child in cards.get_children():
		child.queue_free()

func _on_archipelago_button_toggled(toggled_on: bool) -> void:
	map.show_archipelagos_toggled(toggled_on)

func create_place_card(place)->void:
	var card = DISCOVER_CARROUSEL_ITEM.instantiate()
	card.item_pressed.connect(map.card_pressed)
	cards.add_child(card)
	card.set_data(place)

func async_load_text_search(value: String) -> Array:
	var url = "https://places.decentraland.org/api/places?search=%s&offset=0&limit=50&order_by=most_active&order=desc&with_realms_detail=true" % value
	var promise: Promise = Global.http_requester.request_json(url, HTTPClient.METHOD_GET, "", {})
	var result = await PromiseUtils.async_awaiter(promise)
	if result is PromiseError:
		printerr("Error searching places: ", result.get_error())
		return []

	var json: Dictionary = result.get_string_response_as_json()
	if json.has("data"):
		return json.data
	else:
		return []

func async_load_category(category:int) -> Array:
	var category_string = MapPin.PinCategoryEnum.keys()[category].to_lower()
	var url: String
	if category_string == 'all':
		url = "https://places.decentraland.org/api/places?offset=0&limit=50&order_by=most_active&order=desc&with_realms_detail=true"
	elif category_string == 'live':
		url = "https://events.decentraland.org/api/events/?list=live"
	else:
		url = "https://places.decentraland.org/api/places?offset=0&limit=50&order_by=most_active&order=desc&categories=%s&with_realms_detail=true" % category_string

	var promise: Promise = Global.http_requester.request_json(url, HTTPClient.METHOD_GET, "", {})
	var result = await PromiseUtils.async_awaiter(promise)

	if result is PromiseError:
		printerr("Error request POIs: ", result.get_error())
		return []

	var json: Dictionary = result.get_string_response_as_json()
	if json.has("data"):
		return json.data
	else:
		return []

func _on_size_changed() -> void:
	var window_size: Vector2i = DisplayServer.window_get_size()
	if window_size.x > window_size.y:
		closed_position = Vector2(-landscape_panel_container.size.x, 0)
		search_results.reparent(landscape_container)
		portrait_panel_container.hide()
		landscape_panel_container.visible = !is_closed
	else:
		closed_position = Vector2(0, portrait_panel_container.size.y)
		search_results.reparent(portrait_container)
		portrait_panel_container.visible = !is_closed
		landscape_panel_container.hide()
	
	if is_closed:
		search_results_container.position = closed_position
	else:
		search_results_container.position = Vector2.ZERO
