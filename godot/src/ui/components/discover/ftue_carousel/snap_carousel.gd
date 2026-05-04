class_name SnapCarousel
extends Control

signal card_changed(index: int)

enum Mode { FTUE, BANNER }

@export var mode: Mode = Mode.FTUE
@export var center_card_size: Vector2 = Vector2(600, 480)
@export var side_card_size: Vector2 = Vector2(550, 440)
@export var card_gap: float = 20.0
@export var swipe_threshold: float = 80.0
@export var tween_duration: float = 0.3

var _cards: Array[Control] = []
var _current_index: int = 0
var _is_touching: bool = false
var _touch_start_x: float = 0.0
var _drag_offset: float = 0.0
var _is_dragging: bool = false
var _active_tween: Tween = null
var _dot_indicators: Array[Control] = []

@onready var dots_container: HBoxContainer = %DotsContainer


func _ready() -> void:
	clip_contents = true
	_update_dots_visibility()


func get_current_index() -> int:
	return _current_index


func get_current_card() -> Control:
	if _current_index >= 0 and _current_index < _cards.size():
		return _cards[_current_index]
	return null


func get_card_count() -> int:
	return _cards.size()


func set_cards(cards: Array[Control]) -> void:
	for card in _cards:
		if is_instance_valid(card):
			card.queue_free()
	_cards.clear()
	_clear_dots()

	for card in cards:
		add_child(card)
		card.mouse_filter = Control.MOUSE_FILTER_IGNORE
		_cards.append(card)

	_current_index = 0
	_create_dots()
	_update_dots_visibility()
	_layout_cards_immediate()

	if _cards.size() > 0:
		card_changed.emit(_current_index)


func select_card(index: int) -> void:
	if _cards.is_empty():
		return
	index = clampi(index, 0, _cards.size() - 1)
	if index == _current_index:
		return
	_current_index = index
	_animate_cards()
	_update_dot_selection()
	card_changed.emit(_current_index)


func _gui_input(event: InputEvent) -> void:
	if _cards.size() <= 1:
		return

	if event is InputEventScreenTouch:
		if event.pressed:
			_is_touching = true
			_is_dragging = false
			_touch_start_x = event.position.x
			_drag_offset = 0.0
			if _active_tween and _active_tween.is_valid():
				_active_tween.kill()
		else:
			if _is_dragging:
				_on_swipe_end()
				accept_event()
			_is_touching = false
			_is_dragging = false

	elif event is InputEventScreenDrag:
		if _is_touching:
			var offset_x: float = event.position.x - _touch_start_x
			if not _is_dragging and abs(offset_x) > 30:
				_is_dragging = true
			if _is_dragging:
				_drag_offset = offset_x
				_layout_cards_with_offset(_drag_offset)
				accept_event()


func _on_swipe_end() -> void:
	var new_index := _current_index
	if abs(_drag_offset) > swipe_threshold:
		if _drag_offset < 0 and _current_index < _cards.size() - 1:
			new_index = _current_index + 1
		elif _drag_offset > 0 and _current_index > 0:
			new_index = _current_index - 1

	_drag_offset = 0.0
	if new_index != _current_index:
		_current_index = new_index
		_update_dot_selection()
		card_changed.emit(_current_index)
	_animate_cards()


func _get_card_target_x(card_index: int) -> float:
	var center_x: float = (size.x - center_card_size.x) / 2.0
	var offset_from_center: int = card_index - _current_index

	if offset_from_center == 0:
		return center_x
	if offset_from_center < 0:
		return center_x + offset_from_center * (side_card_size.x + card_gap)
	return (
		center_x
		+ center_card_size.x
		+ card_gap
		+ (offset_from_center - 1) * (side_card_size.x + card_gap)
	)


func _get_card_target_size(card_index: int) -> Vector2:
	if card_index == _current_index:
		return center_card_size
	return side_card_size


func _get_card_target_y(card_index: int) -> float:
	var card_size := _get_card_target_size(card_index)
	return (size.y - card_size.y) / 2.0


func _layout_cards_immediate() -> void:
	for i in _cards.size():
		var card := _cards[i]
		var target_size := _get_card_target_size(i)
		var target_x := _get_card_target_x(i)
		var target_y := _get_card_target_y(i)
		card.position = Vector2(target_x, target_y)
		card.size = target_size
		card.custom_minimum_size = target_size


func _layout_cards_with_offset(offset: float) -> void:
	for i in _cards.size():
		var card := _cards[i]
		var target_size := _get_card_target_size(i)
		var target_x := _get_card_target_x(i) + offset
		var target_y := _get_card_target_y(i)
		card.position = Vector2(target_x, target_y)
		card.size = target_size
		card.custom_minimum_size = target_size


func _animate_cards() -> void:
	if _active_tween and _active_tween.is_valid():
		_active_tween.kill()

	_active_tween = create_tween().set_parallel(true)
	_active_tween.set_ease(Tween.EASE_OUT)
	_active_tween.set_trans(Tween.TRANS_QUART)

	for i in _cards.size():
		var card := _cards[i]
		var target_size := _get_card_target_size(i)
		var target_x := _get_card_target_x(i)
		var target_y := _get_card_target_y(i)

		_active_tween.tween_property(
			card, "position", Vector2(target_x, target_y), tween_duration
		)
		_active_tween.tween_property(card, "size", target_size, tween_duration)
		_active_tween.tween_property(
			card, "custom_minimum_size", target_size, tween_duration
		)


# --- Dot indicators ---


func _create_dots() -> void:
	if not is_instance_valid(dots_container):
		return
	_clear_dots()
	for i in _cards.size():
		var dot := _create_dot(i == _current_index)
		dots_container.add_child(dot)
		_dot_indicators.append(dot)


func _clear_dots() -> void:
	_dot_indicators.clear()
	if not is_instance_valid(dots_container):
		return
	for child in dots_container.get_children():
		child.queue_free()


func _create_dot(is_active: bool) -> Control:
	var dot := Panel.new()
	dot.custom_minimum_size = Vector2(12, 12)
	var style := StyleBoxFlat.new()
	style.corner_radius_top_left = 6
	style.corner_radius_top_right = 6
	style.corner_radius_bottom_left = 6
	style.corner_radius_bottom_right = 6
	if is_active:
		style.bg_color = Color(1, 1, 1, 1)
	else:
		style.bg_color = Color(1, 1, 1, 0.4)
	dot.add_theme_stylebox_override("panel", style)
	return dot


func _update_dot_selection() -> void:
	for i in _dot_indicators.size():
		var dot := _dot_indicators[i]
		var style: StyleBoxFlat = dot.get_theme_stylebox("panel") as StyleBoxFlat
		if style:
			style.bg_color = (
				Color(1, 1, 1, 1) if i == _current_index else Color(1, 1, 1, 0.4)
			)


func _update_dots_visibility() -> void:
	if is_instance_valid(dots_container):
		dots_container.visible = mode == Mode.BANNER
