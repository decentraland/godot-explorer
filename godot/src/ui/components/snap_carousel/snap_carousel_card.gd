class_name SnapCarouselCard
extends Control

signal image_loaded
signal jump_in_pressed

enum CardMode { FTUE, BANNER }

const FTUE_SELECTED_SIZE := SnapCarousel.FTUE_SELECTED_SIZE
const FTUE_UNSELECTED_SIZE := SnapCarousel.FTUE_UNSELECTED_SIZE
const BANNER_SELECTED_SIZE := SnapCarousel.BANNER_SELECTED_SIZE
const BANNER_UNSELECTED_SIZE := SnapCarousel.BANNER_UNSELECTED_SIZE

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
	Global.metrics.track_click_button(
		"JUMP_IN", "DISCOVER_BANNER_CLICK", JSON.stringify({"place_id": _data.get("id", "")})
	)
	jump_in_pressed.emit()
	if PlacesHelper.is_world(_data):
		var pos_realm := PlacesHelper.get_position_and_realm(_data)
		Global.async_join_world(pos_realm[1])
	else:
		var pos_realm := PlacesHelper.get_position_and_realm(_data)
		Global.async_teleport_to(pos_realm[0], pos_realm[1])


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
		button_jump_in.visible = _card_mode == CardMode.BANNER


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
