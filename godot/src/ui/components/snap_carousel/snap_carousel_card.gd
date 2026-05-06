class_name SnapCarouselCard
extends Control

signal image_loaded
signal jump_in_pressed

enum CardMode { FTUE, BANNER }

const FTUE_SELECTED_SIZE := Vector2(600, 480)
const FTUE_UNSELECTED_SIZE := Vector2(550, 440)
const BANNER_SELECTED_SIZE := Vector2(624, 350)
const BANNER_UNSELECTED_SIZE := Vector2(530, 300)

const FTUE_BORDER_RADIUS := 12
const BANNER_BORDER_RADIUS := 8
const TAP_THRESHOLD := 20.0

var _data: Dictionary = {}
var _card_mode: int = CardMode.FTUE
var _is_selected: bool = false

var _touch_start: Vector2 = Vector2.ZERO
var _touch_active: bool = false

@onready var async_image: AsyncImage = %AsyncImage
@onready var button_jump_in: Button = %Button_JumpIn_Banner


func _ready() -> void:
	button_jump_in.pressed.connect(_do_banner_jump_in)
	async_image.image_loaded.connect(func(): image_loaded.emit())
	_update_jump_in_visibility()


func _gui_input(event: InputEvent) -> void:
	if _card_mode != CardMode.BANNER or not _is_selected:
		return
	if event is InputEventScreenTouch:
		if event.pressed:
			_touch_start = event.position
			_touch_active = true
		elif _touch_active:
			_touch_active = false
			if event.position.distance_to(_touch_start) < TAP_THRESHOLD:
				accept_event()
				_do_banner_jump_in()
	elif event is InputEventScreenDrag:
		if _touch_active and event.position.distance_to(_touch_start) >= TAP_THRESHOLD:
			_touch_active = false


func _do_banner_jump_in() -> void:
	if _data.is_empty():
		return
	var scene_name: String = _data.get("title", "")
	# Debug print — remove after testing
	print("[FtueCard] BANNER jump_in: ", scene_name)
	Global.metrics.track_click_button(
		"JUMP_IN", "DISCOVER_BANNER_CLICK", JSON.stringify({"scene": scene_name})
	)
	jump_in_pressed.emit()
	if _is_world(_data):
		var pos_realm := _get_position_and_realm(_data)
		Global.async_join_world(pos_realm[1])
	else:
		var pos_realm := _get_position_and_realm(_data)
		Global.async_teleport_to(pos_realm[0], pos_realm[1])


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
	return [_parse_position(item_data), r]


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


func is_image_ready() -> bool:
	return async_image.is_image_ready()


func set_card_mode(mode: int) -> void:
	_card_mode = mode
	_apply_size()
	if is_instance_valid(async_image):
		async_image.border_radius = (
			FTUE_BORDER_RADIUS if mode == CardMode.FTUE else BANNER_BORDER_RADIUS
		)
	_update_jump_in_visibility()


func set_selected(selected: bool) -> void:
	_is_selected = selected
	_apply_size()
	_update_jump_in_visibility()


func get_target_size() -> Vector2:
	if _card_mode == CardMode.FTUE:
		return FTUE_SELECTED_SIZE if _is_selected else FTUE_UNSELECTED_SIZE
	return BANNER_SELECTED_SIZE if _is_selected else BANNER_UNSELECTED_SIZE


func _apply_size() -> void:
	custom_minimum_size = get_target_size()


func _update_jump_in_visibility() -> void:
	if is_instance_valid(button_jump_in):
		button_jump_in.visible = _card_mode == CardMode.BANNER and _is_selected


func set_data(place_data: Dictionary) -> void:
	_data = place_data
	var image_url: String = place_data.get("image", place_data.get("imageUrl", ""))
	async_image.load_from_url(image_url)


func get_title() -> String:
	return _data.get("title", "")


func get_creator() -> String:
	var contact_name: String = _data.get("contact_name", "")
	if contact_name.is_empty():
		contact_name = _data.get("owner", "")
	return contact_name


func get_place_data() -> Dictionary:
	return _data
