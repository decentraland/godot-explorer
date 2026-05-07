@tool
class_name SnapCarousel
extends VBoxContainer

signal card_changed(index: int)
signal all_cards_loaded
signal items_loaded(places: Array[Dictionary])

enum Mode { FTUE, BANNER }

const FTUE_SELECTED_SIZE := Vector2(600, 480)
const FTUE_UNSELECTED_SIZE := Vector2(550, 440)
const BANNER_SELECTED_SIZE := Vector2(624, 350)
const BANNER_UNSELECTED_SIZE := Vector2(530, 300)
const CARD_SCENE_PATH = "res://src/ui/components/snap_carousel/snap_carousel_card.tscn"
const FeaturedDataProvider = preload(
	"res://src/ui/components/snap_carousel/featured_data_provider.gd"
)

@export var mode: Mode = Mode.FTUE:
	set(v):
		mode = v
		if is_inside_tree():
			_apply_mode()
			_layout_cards()

@export var selected_index: int = 1:
	set(v):
		var old := selected_index
		selected_index = v
		if is_inside_tree():
			_layout_cards()
			if old != selected_index:
				_update_dot_selection()

## Minimum drag distance to advance to next/previous card
@export var swipe_threshold: float = 60.0
## Snap animation duration in seconds
@export var snap_duration: float = 0.25
@export var auto_fetch: bool = false

var _is_touching: bool = false
var _touch_start_x: float = 0.0
var _drag_offset: float = 0.0
var _is_dragging: bool = false
var _is_animating: bool = false
var _snap_tween: Tween = null
var _dot_indicators: Array[Control] = []

@onready var card_area: Control = $CardArea
@onready var item_container: HBoxContainer = %HBoxContainer_Items
@onready var dots_container: HBoxContainer = %DotsContainer


func _ready() -> void:
	clip_contents = true
	_apply_mode()

	if not Engine.is_editor_hint() and auto_fetch:
		fetch()


func _process(_delta: float) -> void:
	if Engine.is_editor_hint():
		return
	# Keep selected card centered while sizes are animating
	if _is_animating and not _is_dragging:
		var offset_x: float = _get_scroll_offset_for_index(selected_index)
		item_container.position.x = offset_x


func _get_selected_size() -> Vector2:
	return FTUE_SELECTED_SIZE if mode == Mode.FTUE else BANNER_SELECTED_SIZE


func _get_unselected_size() -> Vector2:
	return FTUE_UNSELECTED_SIZE if mode == Mode.FTUE else BANNER_UNSELECTED_SIZE


func _apply_mode() -> void:
	if is_instance_valid(dots_container):
		dots_container.visible = mode == Mode.BANNER
	if is_instance_valid(item_container):
		item_container.add_theme_constant_override("separation", 32 if mode == Mode.FTUE else 16)
		for i in item_container.get_child_count():
			var card: Control = item_container.get_child(i)
			if card.has_method("set_card_mode"):
				card.set_card_mode(mode)
	if is_instance_valid(card_area):
		card_area.custom_minimum_size.y = _get_selected_size().y
	_apply_card_sizes(false)


# gdlint: ignore=async-function-name
func fetch() -> void:
	var places := await FeaturedDataProvider.async_fetch_ftue_places()
	if not is_instance_valid(self) or places.is_empty():
		return
	set_items(places)


# gdlint: ignore=async-function-name
func set_items(data_list: Array[Dictionary]) -> void:
	if not is_instance_valid(item_container):
		return

	# Clear existing cards
	for child in item_container.get_children():
		item_container.remove_child(child)
		child.queue_free()

	# Instantiate cards with correct mode and size from the start
	var idx := 1 if data_list.size() >= 3 else 0
	var card_scene: PackedScene = load(CARD_SCENE_PATH)
	for i in data_list.size():
		var card: Control = card_scene.instantiate()
		item_container.add_child(card)
		card.set_card_mode(mode)
		card.set_selected(i == idx)
		card.set_data(data_list[i])

	rebuild_dots()
	watch_card_loading()
	selected_index = idx
	items_loaded.emit(data_list)

	await get_tree().process_frame
	if not is_instance_valid(self):
		return
	_layout_cards()


func get_current_index() -> int:
	return selected_index


func get_card_count() -> int:
	if not is_instance_valid(item_container):
		return 0
	return item_container.get_child_count()


func get_cards() -> Array:
	if not is_instance_valid(item_container):
		return []
	return item_container.get_children()


func _get_card_separation() -> float:
	if not is_instance_valid(item_container):
		return 32.0
	return float(item_container.get_theme_constant("separation"))


func _get_visible_width() -> float:
	if is_instance_valid(card_area):
		return card_area.size.x
	return size.x


func _get_scroll_offset_for_index(index: int) -> float:
	# Calculate the x-offset of item_container so that card[index] is centered
	if not is_instance_valid(item_container) or item_container.get_child_count() == 0:
		return 0.0
	if index < 0 or index >= item_container.get_child_count():
		return 0.0
	var card: Control = item_container.get_child(index)
	var card_center_in_container: float = card.position.x + card.size.x / 2.0
	return (_get_visible_width() / 2.0) - card_center_in_container


func _layout_cards() -> void:
	if not is_instance_valid(item_container):
		return
	_apply_card_sizes(false)
	var offset_x: float = _get_scroll_offset_for_index(selected_index) + _drag_offset
	item_container.position.x = offset_x


func _apply_card_sizes(animate: bool) -> void:
	if not is_instance_valid(item_container):
		return
	for i in item_container.get_child_count():
		var card: Control = item_container.get_child(i)
		var is_sel: bool = i == selected_index
		if card.has_method("set_selected"):
			card.set_selected(is_sel)
		var target_size: Vector2
		if card.has_method("get_target_size"):
			target_size = card.get_target_size()
		else:
			target_size = _get_selected_size() if is_sel else _get_unselected_size()

		if animate and not Engine.is_editor_hint():
			if _snap_tween and _snap_tween.is_valid():
				_snap_tween.tween_property(card, "custom_minimum_size", target_size, snap_duration)
		else:
			card.custom_minimum_size = target_size


func _gui_input(event: InputEvent) -> void:
	if Engine.is_editor_hint():
		return
	if get_card_count() <= 1:
		return

	if event is InputEventScreenTouch:
		if event.pressed:
			_is_touching = true
			_is_dragging = false
			_touch_start_x = event.position.x
			_drag_offset = 0.0
			if _snap_tween and _snap_tween.is_valid():
				_snap_tween.kill()
		else:
			if _is_dragging:
				_on_touch_released()
				accept_event()
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
	var max_index: int = get_card_count() - 1
	var new_index: int = selected_index

	if abs(_drag_offset) > swipe_threshold:
		if _drag_offset < 0:
			new_index = mini(selected_index + 1, max_index)
		else:
			new_index = maxi(selected_index - 1, 0)

	_drag_offset = 0.0

	if new_index != selected_index:
		selected_index = new_index
		card_changed.emit(selected_index)
		_update_dot_selection()

	_animate_to_selected()


func _animate_to_selected() -> void:
	if not is_instance_valid(item_container):
		return

	if _snap_tween and _snap_tween.is_valid():
		_snap_tween.kill()

	_is_animating = true
	_snap_tween = create_tween().set_parallel(true)
	_snap_tween.set_ease(Tween.EASE_OUT)
	_snap_tween.set_trans(Tween.TRANS_SINE)

	# Animate card sizes
	for i in item_container.get_child_count():
		var card: Control = item_container.get_child(i)
		var target_size := _get_selected_size() if i == selected_index else _get_unselected_size()
		_snap_tween.tween_property(card, "custom_minimum_size", target_size, snap_duration)

	_snap_tween.chain().tween_callback(func(): _is_animating = false)


func watch_card_loading() -> void:
	if not is_instance_valid(item_container):
		return
	for i in item_container.get_child_count():
		var card: Control = item_container.get_child(i)
		if card.has_signal("image_loaded") and not card.is_image_ready():
			card.image_loaded.connect(_check_all_loaded)
	_check_all_loaded()


func _check_all_loaded() -> void:
	if not is_instance_valid(item_container):
		return
	for i in item_container.get_child_count():
		var card: Control = item_container.get_child(i)
		if card.has_method("is_image_ready") and not card.is_image_ready():
			return
	# All loaded — disconnect to avoid duplicate emissions
	for i in item_container.get_child_count():
		var card: Control = item_container.get_child(i)
		if card.has_signal("image_loaded") and card.image_loaded.is_connected(_check_all_loaded):
			card.image_loaded.disconnect(_check_all_loaded)
	all_cards_loaded.emit()


# --- Dot indicators ---


func rebuild_dots() -> void:
	_clear_dots()
	if not is_instance_valid(dots_container) or not is_instance_valid(item_container):
		return
	for i in item_container.get_child_count():
		var dot := _create_dot(i == selected_index)
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
	style.bg_color = Color("E8B9FF") if is_active else Color("00000066")
	dot.add_theme_stylebox_override("panel", style)
	return dot


func _update_dot_selection() -> void:
	for i in _dot_indicators.size():
		var dot := _dot_indicators[i]
		var style: StyleBoxFlat = dot.get_theme_stylebox("panel") as StyleBoxFlat
		if style:
			style.bg_color = Color("E8B9FF") if i == selected_index else Color("00000066")
