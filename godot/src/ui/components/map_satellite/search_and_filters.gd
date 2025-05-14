extends Control

@export var map: MapComponent
@onready var searchbar: PanelContainer = %Searchbar
@onready var archipelago_button: Button = %ArchipelagoButton
@onready var h_box_container_filters: HBoxContainer = %HBoxContainer_Filters
@onready var no_results: VBoxContainer = %NoResults
@onready var cards: BoxContainer = %Cards
@onready var cards_scroll: ScrollContainer = %CardsScroll
@onready var sidebar_container: BoxContainer = %SidebarContainer
@onready var margin_container_sidebar: MarginContainer = $MarginContainer_Sidebar
@onready var panel_container: PanelContainer = %PanelContainer
@onready var flag_component: Control = %Flag

const DISCOVER_CARROUSEL_ITEM = preload("res://src/ui/components/discover/carrousel/discover_carrousel_item.tscn")
const PLACE_CATEGORY_FILTER_BUTTON = preload("res://src/ui/components/map_satellite/place_category_filter_button.tscn")
const SIDE_BAR_WIDTH = 300

var active_filter: int = -1
var filtered_places: Array = []
var poi_places_ids = []
var live_places_ids = []
var map_is_on_top: bool = false
var is_closed: bool = true
var closed_position := Vector2.ZERO

func _ready() -> void:
	get_window().size_changed.connect(self._on_size_changed)
	_on_size_changed()
	margin_container_sidebar.position = closed_position
	
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
	_close_sidebar()

func _close_from_searchbar():
	for child in h_box_container_filters.get_children():
		if child is Button and child.toggle_mode and child.filter_type == active_filter:
			child.button_pressed = false
	_clean_list()
	_close_sidebar(0.4)
	
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
		filtered_places = []
		searchbar.reset()
		_close_sidebar(0.4)
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
	if not sidebar_container.visible:
		sidebar_container.show()
	var duration = .4
	var tween = create_tween()
	tween.tween_property(margin_container_sidebar, "position", Vector2.ZERO, duration).set_trans(Tween.TRANS_LINEAR).set_ease(Tween.EASE_OUT)

func _close_sidebar(duration:float=0.0)->void:
	is_closed = true
	cards_scroll.scroll_horizontal = 0
	cards_scroll.scroll_vertical = 0
	filtered_places = []
	var tween = create_tween()
	tween.tween_property(margin_container_sidebar, "position", closed_position, duration).set_trans(Tween.TRANS_LINEAR).set_ease(Tween.EASE_OUT)
	
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
		closed_position = Vector2(15 - panel_container.size.x, 0)
		panel_container.size_flags_horizontal = Control.SIZE_SHRINK_BEGIN
		panel_container.size_flags_vertical = Control.SIZE_EXPAND_FILL
	else:
		closed_position = Vector2(0, panel_container.size.y - 15)
		panel_container.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		panel_container.size_flags_vertical = Control.SIZE_SHRINK_BEGIN

	if is_closed:
		margin_container_sidebar.position = closed_position
