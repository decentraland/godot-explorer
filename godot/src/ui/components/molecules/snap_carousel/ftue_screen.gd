extends Control

signal ftue_completed
signal jump_in(parcel_position: Vector2i, realm_str: String)
signal jump_in_world(realm_str: String)

var _places: Array[Dictionary] = []

@onready var carousel: Control = %SnapCarousel
@onready var label_welcome: RichTextLabel = %Label_Welcome
@onready var button_jump_in: Button = %Button_JumpIn_FTUE
@onready var button_skip: Button = %Button_Skip


func _ready() -> void:
	carousel.card_tapped.connect(_on_card_tapped)
	carousel.items_loaded.connect(_on_items_loaded)
	button_jump_in.pressed.connect(_on_button_jump_in_pressed)
	button_skip.pressed.connect(_on_button_skip_pressed)


func set_username(display_name: String) -> void:
	label_welcome.text = (
		"Welcome [color=#B18AFF]@" + display_name + "[/color]\nLet's get you started"
	)


func load_places() -> void:
	carousel.fetch()


func _on_items_loaded(places: Array[Dictionary]) -> void:
	_places.assign(places)
	_track_screen_view()


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
