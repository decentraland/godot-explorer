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
	# Debug print — remove after testing
	print("[FtueScreen] FTUE jump_in: ", scene_name)
	Global.metrics.track_click_button(
		"JUMP_IN", "DISCOVER_FTUE_CLICK", JSON.stringify({"scene": scene_name})
	)
	ftue_completed.emit()
	_do_jump_in(place)


func _on_button_skip_pressed() -> void:
	var scene_name := ""
	if not _places.is_empty():
		scene_name = _places[carousel.get_current_index()].get("title", "")
	# Debug print — remove after testing
	print("[FtueScreen] FTUE skip, current scene: ", scene_name)
	Global.metrics.track_click_button(
		"SKIP", "DISCOVER_FTUE_CLICK", JSON.stringify({"scene": scene_name})
	)
	ftue_completed.emit()


func _do_jump_in(place_data: Dictionary) -> void:
	if _is_world(place_data):
		var pos_realm := _get_position_and_realm(place_data)
		jump_in_world.emit(pos_realm[1])
		return
	var pos_realm := _get_position_and_realm(place_data)
	jump_in.emit(pos_realm[0], pos_realm[1])


static func _is_world(item_data: Dictionary) -> bool:
	if item_data.get("world", false):
		return true
	var server = item_data.get("server", null)
	if server == null:
		return false
	var s := str(server).strip_edges()
	return s != "" and s != "main"


static func _get_position_and_realm(item_data: Dictionary) -> Array:
	var server = item_data.get("server", null)
	var world_name = item_data.get("world_name", null)
	var r: String
	if server and str(server) != "main":
		r = str(server)
		if not r.ends_with(".dcl.eth"):
			r = r + ".dcl.eth"
	elif item_data.get("world", false) and world_name:
		r = str(world_name)
		if not r.ends_with(".dcl.eth"):
			r = r + ".dcl.eth"
	else:
		r = DclUrls.main_realm()
	var pos := _parse_position(item_data)
	return [pos, r]


static func _parse_position(item_data: Dictionary) -> Vector2i:
	var coords = item_data.get("coordinates", null)
	var pos_arr = item_data.get("position", null)
	var base_pos = item_data.get("base_position", null)
	if coords is Array and coords.size() >= 2:
		return Vector2i(int(coords[0]), int(coords[1]))
	if pos_arr is Array and pos_arr.size() >= 2:
		return Vector2i(int(pos_arr[0]), int(pos_arr[1]))
	if item_data.get("x") != null and item_data.get("y") != null:
		return Vector2i(int(item_data.x), int(item_data.y))
	if base_pos:
		var parts = str(base_pos).split(",")
		if parts.size() >= 2:
			return Vector2i(int(parts[0]), int(parts[1]))
	return Vector2i.ZERO
