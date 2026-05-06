extends Control

signal ftue_completed
signal jump_in(parcel_position: Vector2i, realm_str: String)
signal jump_in_world(realm_str: String)

var _places: Array[Dictionary] = []

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


func _on_all_cards_loaded() -> void:
	label_title.show()
	hbox_creator.show()
	title_skeleton.hide()
	creator_skeleton.hide()


func _on_card_changed(index: int) -> void:
	if index < 0 or index >= _places.size():
		return
	var place: Dictionary = _places[index]
	label_title.text = place.get("title", "")
	var creator: String = place.get("contact_name", "")
	if creator.is_empty():
		creator = place.get("owner", "")
	label_creator.text = creator


func _on_button_jump_in_pressed() -> void:
	if _places.is_empty():
		return
	var place: Dictionary = _places[carousel.get_current_index()]
	var scene_name: String = place.get("title", "")
	Global.metrics.track_click_button(
		"JUMP_IN", "DISCOVER_FTUE_CLICK", JSON.stringify({"scene": scene_name})
	)
	ftue_completed.emit()
	_do_jump_in(place)


func _on_button_skip_pressed() -> void:
	var scene_name := ""
	if not _places.is_empty():
		scene_name = _places[carousel.get_current_index()].get("title", "")
	Global.metrics.track_click_button(
		"SKIP", "DISCOVER_FTUE_CLICK", JSON.stringify({"scene": scene_name})
	)
	ftue_completed.emit()


func _do_jump_in(place_data: Dictionary) -> void:
	if PlacesHelper.is_world(place_data):
		var pos_realm := PlacesHelper.get_position_and_realm(place_data)
		jump_in_world.emit(pos_realm[1])
		return
	var pos_realm := PlacesHelper.get_position_and_realm(place_data)
	jump_in.emit(pos_realm[0], pos_realm[1])
