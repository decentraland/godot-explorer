extends Control

signal ftue_completed
signal jump_in(parcel_position: Vector2i, realm_str: String)
signal jump_in_world(realm_str: String)

var _places: Array[Dictionary] = []
var _cards_loaded: bool = false

@onready var carousel: Control = %SnapCarousel
@onready var label_title: Label = %Label_Title
@onready var label_creator: Label = %Label_Creator
@onready var label_nickname: Label = %Label_NickNameFTUE
@onready var button_jump_in: Button = %Button_JumpIn_FTUE
@onready var button_skip: Button = %Button_Skip
@onready var title_skeleton: Control = %TitleSkeleton
@onready var creator_skeleton: Control = %CreatorSkeleton
@onready var hbox_creator: HBoxContainer = %HBoxContainer_Creator


func _ready() -> void:
	carousel.card_changed.connect(_on_card_changed)
	carousel.card_tapped.connect(_on_card_tapped)
	carousel.all_cards_loaded.connect(_on_all_cards_loaded)
	carousel.items_loaded.connect(_on_items_loaded)
	button_jump_in.pressed.connect(_on_button_jump_in_pressed)
	button_skip.pressed.connect(_on_button_skip_pressed)
	label_title.hide()
	hbox_creator.hide()
	title_skeleton.show()
	creator_skeleton.show()


func set_username(display_name: String) -> void:
	label_nickname.text = display_name


func load_places() -> void:
	carousel.fetch()


func _on_items_loaded(places: Array[Dictionary]) -> void:
	_places.assign(places)
	_on_card_changed(carousel.selected_index)
	_track_screen_view()


func _on_all_cards_loaded() -> void:
	_cards_loaded = true
	label_title.show()
	title_skeleton.hide()
	creator_skeleton.hide()
	_update_creator(carousel.selected_index)


func _on_card_changed(index: int) -> void:
	if index < 0 or index >= _places.size():
		return
	var place: Dictionary = _places[index]
	label_title.text = place.get("title", "")
	_update_creator(index)


# Show the "by <creator>" row only when the place has a real creator name.
# The destinations API returns `contact_name: null` for worlds/places without
# one (e.g. the Kickoff world), and the only other identifier is the raw 0x
# `owner` address. Assigning that null to a typed String used to crash
# _on_card_changed (leaking the "Creator" placeholder); the raw-owner fallback
# only printed a wallet address. Hide the whole row instead when there's no name.
func _update_creator(index: int) -> void:
	if index < 0 or index >= _places.size():
		hbox_creator.hide()
		return
	var raw = _places[index].get("contact_name")
	var creator := "" if raw == null else str(raw)
	if creator.strip_edges().is_empty():
		hbox_creator.hide()
		return
	label_creator.text = creator
	if _cards_loaded:
		hbox_creator.show()


func _on_card_tapped(_index: int) -> void:
	_on_button_jump_in_pressed()


func _on_button_jump_in_pressed() -> void:
	if _places.is_empty():
		return
	var index = carousel.get_current_index()
	var place: Dictionary = _places[index]
	(
		Global
		. metrics
		. track_click_button(
			"JUMP_IN",
			"DISCOVER_FTUE",
			JSON.stringify({"place_id": place.get("id", ""), "position": index}),
		)
	)
	ftue_completed.emit()
	_do_jump_in(place)


func _on_button_skip_pressed() -> void:
	Global.metrics.track_click_button("SKIP", "DISCOVER_FTUE", "")
	ftue_completed.emit()


func _do_jump_in(place_data: Dictionary) -> void:
	if PlacesHelper.is_world(place_data):
		var pos_realm := PlacesHelper.get_position_and_realm(place_data)
		jump_in_world.emit(pos_realm[1])
		return
	var pos_realm := PlacesHelper.get_position_and_realm(place_data)
	jump_in.emit(pos_realm[0], pos_realm[1])


func _track_screen_view() -> void:
	var carousel_items = []
	for i in _places.size():
		carousel_items.append({"position": i, "place_id": _places[i].get("id", "")})
	Global.metrics.track_screen_viewed(
		"DISCOVER_FTUE", JSON.stringify({"carousel": carousel_items})
	)
