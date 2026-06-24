@tool
class_name SnapCarousel
extends VBoxContainer

signal card_changed(index: int)
signal card_tapped(index: int)
signal all_cards_loaded
signal items_loaded(places: Array[Dictionary])

enum Mode { FTUE, BANNER }

const FTUE_SELECTED_SIZE = Vector2(624, 350)
const FTUE_UNSELECTED_SIZE = Vector2(530, 300)
const BANNER_SELECTED_SIZE = Vector2(624, 350)
const BANNER_UNSELECTED_SIZE = Vector2(530, 300)
const CARD_SCENE_PATH = "res://src/ui/components/molecules/snap_carousel/snap_carousel_card.tscn"
const FeaturedDataProvider = preload(
	"res://src/ui/components/molecules/snap_carousel/featured_data_provider.gd"
)

## Title shown above the carousel, left-aligned with the selected card.
## Requires a MarginContainer with unique name "TitleContainer" as first child
## (containing the Label) — add it via the editor.
@export var show_title: bool = false:
	set(value):
		show_title = value
		if is_node_ready() and is_instance_valid(_title_container):
			_title_container.visible = value

@export var mode: Mode = Mode.FTUE:
	set(v):
		mode = v
		if is_inside_tree():
			_apply_mode()
			_layout_cards()

@export var selected_index: int = 0

## Minimum drag distance to advance to next/previous card
@export var swipe_threshold: float = 60.0
## Snap animation duration in seconds
@export var snap_duration: float = 0.25
@export var auto_fetch: bool = false
@export var fetch_tag: String = "featured"
@export var auto_scroll: bool = false
@export var auto_scroll_interval: float = 3.0

var _title_container: MarginContainer = null

var _cards: Array = []
var _auto_scroll_timer: Timer = null
var _is_touching: bool = false
var _touch_start_x: float = 0.0
var _drag_offset: float = 0.0
var _is_dragging: bool = false
var _is_animating: bool = false
var _snap_tween: Tween = null
var _dot_indicators: Array[Control] = []

@onready var card_area: Control = $CardArea
@onready var card_container: Control = %CardContainer
@onready var dots_container: HBoxContainer = %DotsContainer


func _ready() -> void:
	clip_contents = true
	_title_container = get_node_or_null("%TitleContainer")
	if is_instance_valid(_title_container):
		_title_container.visible = show_title
	_apply_mode()

	if not Engine.is_editor_hint():
		_setup_auto_scroll_timer()
		if auto_fetch:
			fetch()


func _get_selected_size() -> Vector2:
	return FTUE_SELECTED_SIZE if mode == Mode.FTUE else BANNER_SELECTED_SIZE


func _get_unselected_size() -> Vector2:
	return FTUE_UNSELECTED_SIZE if mode == Mode.FTUE else BANNER_UNSELECTED_SIZE


func _get_card_separation() -> float:
	return 32.0 if mode == Mode.FTUE else 16.0


func _apply_mode() -> void:
	if is_instance_valid(dots_container):
		dots_container.visible = mode == Mode.BANNER
	for card in _cards:
		if card.has_method("set_card_mode"):
			card.set_card_mode(mode)
	if is_instance_valid(card_area):
		card_area.custom_minimum_size.y = _get_selected_size().y
	_layout_cards()


# gdlint: ignore=async-function-name
func fetch() -> void:
	var places = await FeaturedDataProvider.async_fetch_places(fetch_tag)
	if not is_instance_valid(self) or places.is_empty():
		return
	set_items(places)


# gdlint: ignore=async-function-name
func set_items(data_list: Array[Dictionary]) -> void:
	if not is_instance_valid(card_container):
		return

	_cards.clear()
	for child in card_container.get_children():
		card_container.remove_child(child)
		child.queue_free()

	var card_scene: PackedScene = load(CARD_SCENE_PATH)
	for i in data_list.size():
		var card: Control = card_scene.instantiate()
		card_container.add_child(card)
		card._card_index = i
		card.set_card_mode(mode)
		card.set_data(data_list[i])
		_cards.append(card)

	selected_index = 0
	rebuild_dots()
	watch_card_loading()
	items_loaded.emit(data_list)

	await get_tree().process_frame
	if not is_instance_valid(self):
		return
	_layout_cards()
	_restart_auto_scroll()


func get_current_index() -> int:
	return selected_index


func get_card_count() -> int:
	return _cards.size()


func get_cards() -> Array:
	return _cards.duplicate()


func _get_visible_width() -> float:
	if is_instance_valid(card_area):
		return card_area.size.x
	return size.x


func _wrap_index(i: int) -> int:
	var count = _cards.size()
	if count == 0:
		return 0
	return ((i % count) + count) % count


# --- Layout: manual positioning of up to 3 visible cards ---


func _layout_cards() -> void:
	var count = _cards.size()
	if count == 0:
		return

	var sel_size = _get_selected_size()
	var unsel_size = _get_unselected_size()
	var sep = _get_card_separation()
	var area_w = _get_visible_width()
	var area_h = sel_size.y

	# Hide all cards
	for card in _cards:
		card.visible = false

	var center_x = (area_w - sel_size.x) / 2.0 + _drag_offset

	# Align title left edge with the selected card (use base center_x, not drag offset).
	if is_instance_valid(_title_container):
		_title_container.add_theme_constant_override("margin_left", int((area_w - sel_size.x) / 2.0))

	# Selected card (center)
	var sel_card = _cards[selected_index]
	sel_card.visible = true
	sel_card.custom_minimum_size = sel_size
	sel_card.size = sel_size
	sel_card.position = Vector2(center_x, 0)
	if sel_card.has_method("set_selected"):
		sel_card.set_selected(true)

	if count < 2:
		return

	# Previous card (left — wraps to last when selected is 0)
	var prev_i = _wrap_index(selected_index - 1)
	var prev_card = _cards[prev_i]
	prev_card.visible = true
	prev_card.custom_minimum_size = unsel_size
	prev_card.size = unsel_size
	prev_card.position = Vector2(center_x - sep - unsel_size.x, (area_h - unsel_size.y) / 2.0)
	if prev_card.has_method("set_selected"):
		prev_card.set_selected(false)

	# Next card (right — wraps to first when selected is last)
	var next_i = _wrap_index(selected_index + 1)
	if next_i == prev_i:
		return
	var next_card = _cards[next_i]
	next_card.visible = true
	next_card.custom_minimum_size = unsel_size
	next_card.size = unsel_size
	next_card.position = Vector2(center_x + sel_size.x + sep, (area_h - unsel_size.y) / 2.0)
	if next_card.has_method("set_selected"):
		next_card.set_selected(false)

	# During drag with 4+ cards, show the entering card on the drag side
	if _is_dragging and count > 3:
		if _drag_offset < 0:
			var nn_i = _wrap_index(selected_index + 2)
			var nn_card = _cards[nn_i]
			nn_card.visible = true
			nn_card.custom_minimum_size = unsel_size
			nn_card.size = unsel_size
			nn_card.position = Vector2(
				center_x + sel_size.x + sep + unsel_size.x + sep,
				(area_h - unsel_size.y) / 2.0,
			)
			if nn_card.has_method("set_selected"):
				nn_card.set_selected(false)
		elif _drag_offset > 0:
			var pp_i = _wrap_index(selected_index - 2)
			var pp_card = _cards[pp_i]
			pp_card.visible = true
			pp_card.custom_minimum_size = unsel_size
			pp_card.size = unsel_size
			pp_card.position = Vector2(
				center_x - sep - unsel_size.x - sep - unsel_size.x,
				(area_h - unsel_size.y) / 2.0,
			)
			if pp_card.has_method("set_selected"):
				pp_card.set_selected(false)


# --- Touch / swipe input ---


func _gui_input(event: InputEvent) -> void:
	if Engine.is_editor_hint():
		return
	if _cards.size() <= 1:
		return

	if event is InputEventScreenTouch:
		if event.pressed:
			_is_touching = true
			_is_dragging = false
			_touch_start_x = event.position.x
			_drag_offset = 0.0
			_stop_auto_scroll()
			if _snap_tween and _snap_tween.is_valid():
				_snap_tween.kill()
		else:
			if _is_dragging:
				_on_touch_released()
				accept_event()
			else:
				card_tapped.emit(selected_index)
				_restart_auto_scroll()
			_is_touching = false
			_is_dragging = false
			_drag_offset = 0.0

	elif event is InputEventScreenDrag:
		if _is_touching:
			var delta_x: float = event.position.x - _touch_start_x
			if not _is_dragging and abs(delta_x) > 20:
				_is_dragging = true
			if _is_dragging:
				_drag_offset = delta_x
				_layout_cards()
				accept_event()


func _on_touch_released() -> void:
	var old_index = selected_index
	var new_index = selected_index

	if abs(_drag_offset) > swipe_threshold:
		if _drag_offset < 0:
			new_index = _wrap_index(selected_index + 1)
		else:
			new_index = _wrap_index(selected_index - 1)

	_drag_offset = 0.0

	if new_index != selected_index:
		selected_index = new_index
		card_changed.emit(selected_index)
		_update_dot_selection()

	_animate_to_selected(old_index)


func _animate_to_selected(old_index: int = -1) -> void:
	var count = _cards.size()
	if count == 0:
		return

	if _snap_tween and _snap_tween.is_valid():
		_snap_tween.kill()

	# Determine the old slot (-1=left, 0=center, 1=right) for each card index,
	# so we can detect cards that would cross the center during animation.
	var old_slot = {}
	if old_index >= 0:
		old_slot[_wrap_index(old_index - 1)] = -1
		old_slot[old_index] = 0
		old_slot[_wrap_index(old_index + 1)] = 1

	_is_animating = true
	_snap_tween = create_tween().set_parallel(true)
	_snap_tween.set_ease(Tween.EASE_OUT)
	_snap_tween.set_trans(Tween.TRANS_SINE)

	var sel_size = _get_selected_size()
	var unsel_size = _get_unselected_size()
	var sep = _get_card_separation()
	var area_w = _get_visible_width()
	var area_h = sel_size.y
	var center_x = (area_w - sel_size.x) / 2.0

	# Animate selected card to center
	var sel_card = _cards[selected_index]
	sel_card.visible = true
	_snap_tween.tween_property(sel_card, "size", sel_size, snap_duration)
	_snap_tween.tween_property(sel_card, "custom_minimum_size", sel_size, snap_duration)
	_snap_tween.tween_property(sel_card, "position", Vector2(center_x, 0), snap_duration)

	if count > 1:
		var prev_i = _wrap_index(selected_index - 1)
		var prev_card = _cards[prev_i]
		prev_card.visible = true
		var prev_pos = Vector2(center_x - sep - unsel_size.x, (area_h - unsel_size.y) / 2.0)
		# Slide from current position only if card was adjacent or center.
		# If it was on the opposite side (slot 1 = right, going to left),
		# reposition off-screen on the left and slide in.
		var prev_old_slot = old_slot.get(prev_i, -99)
		if prev_old_slot == -99 or prev_old_slot == 1:
			prev_card.position = Vector2(prev_pos.x - unsel_size.x - sep, prev_pos.y)
		_snap_tween.tween_property(prev_card, "position", prev_pos, snap_duration)
		_snap_tween.tween_property(prev_card, "size", unsel_size, snap_duration)
		_snap_tween.tween_property(prev_card, "custom_minimum_size", unsel_size, snap_duration)

	if count > 2:
		var next_i = _wrap_index(selected_index + 1)
		var next_card = _cards[next_i]
		next_card.visible = true
		var next_pos = Vector2(center_x + sel_size.x + sep, (area_h - unsel_size.y) / 2.0)
		# If card was on the opposite side (slot -1 = left, going to right),
		# reposition off-screen on the right and slide in.
		var next_old_slot = old_slot.get(next_i, -99)
		if next_old_slot == -99 or next_old_slot == -1:
			next_card.position = Vector2(next_pos.x + unsel_size.x + sep, next_pos.y)
		_snap_tween.tween_property(next_card, "position", next_pos, snap_duration)
		_snap_tween.tween_property(next_card, "size", unsel_size, snap_duration)
		_snap_tween.tween_property(next_card, "custom_minimum_size", unsel_size, snap_duration)

	_snap_tween.chain().tween_callback(_on_snap_finished)


func _on_snap_finished() -> void:
	_is_animating = false
	_layout_cards()
	_restart_auto_scroll()


# --- Auto scroll ---


func _setup_auto_scroll_timer() -> void:
	_auto_scroll_timer = Timer.new()
	_auto_scroll_timer.one_shot = true
	_auto_scroll_timer.timeout.connect(_on_auto_scroll_timeout)
	add_child(_auto_scroll_timer)


func _restart_auto_scroll() -> void:
	if not auto_scroll or _cards.size() <= 1:
		return
	if is_instance_valid(_auto_scroll_timer):
		_auto_scroll_timer.start(auto_scroll_interval)


func _stop_auto_scroll() -> void:
	if is_instance_valid(_auto_scroll_timer):
		_auto_scroll_timer.stop()


func _on_auto_scroll_timeout() -> void:
	if _is_dragging or _is_animating or _cards.size() <= 1:
		return
	var old_index = selected_index
	selected_index = _wrap_index(selected_index + 1)
	card_changed.emit(selected_index)
	_update_dot_selection()
	_animate_to_selected(old_index)


# --- Card loading tracking ---


func watch_card_loading() -> void:
	for card in _cards:
		if card.has_signal("image_loaded") and not card.is_image_ready():
			card.image_loaded.connect(_check_all_loaded)
	_check_all_loaded()


func _check_all_loaded() -> void:
	for card in _cards:
		if card.has_method("is_image_ready") and not card.is_image_ready():
			return
	# All loaded — disconnect to avoid duplicate emissions
	for card in _cards:
		if card.has_signal("image_loaded") and card.image_loaded.is_connected(_check_all_loaded):
			card.image_loaded.disconnect(_check_all_loaded)
	all_cards_loaded.emit()


# --- Dot indicators ---


func rebuild_dots() -> void:
	_clear_dots()
	if not is_instance_valid(dots_container):
		return
	for i in _cards.size():
		var dot = _create_dot(i == selected_index)
		dots_container.add_child(dot)
		_dot_indicators.append(dot)


func _clear_dots() -> void:
	_dot_indicators.clear()
	if not is_instance_valid(dots_container):
		return
	for child in dots_container.get_children():
		child.queue_free()


func _create_dot(is_active: bool) -> Control:
	var dot = Panel.new()
	dot.custom_minimum_size = Vector2(12, 12)
	var style = StyleBoxFlat.new()
	style.corner_radius_top_left = 6
	style.corner_radius_top_right = 6
	style.corner_radius_bottom_left = 6
	style.corner_radius_bottom_right = 6
	style.bg_color = Color("E8B9FF") if is_active else Color("00000066")
	dot.add_theme_stylebox_override("panel", style)
	return dot


func _update_dot_selection() -> void:
	for i in _dot_indicators.size():
		var dot = _dot_indicators[i]
		var style: StyleBoxFlat = dot.get_theme_stylebox("panel") as StyleBoxFlat
		if style:
			style.bg_color = Color("E8B9FF") if i == selected_index else Color("00000066")
