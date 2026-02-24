class_name DraggableBottomSheet
extends VBoxContainer

## Generic draggable bottom sheet with three states: PEEK, HALF, FULL.
## Swipe up: PEEK → HALF → FULL. Swipe down: FULL → HALF → PEEK.
## PEEK always shows a small grab area so the user can drag back up.
## Set content_scene to inject a scene, half_height_ratio for HALF position,
## and closeable to control whether the close button hides or returns to HALF.

signal close_requested

enum DragState { PEEK, HALF, FULL }
enum DragGesture { IDLE, UP, DOWN }

const _TWEEN_DURATION := 0.2
const _DRAG_THRESHOLD := 50.0
const _PEEK_HEIGHT := 80.0

## Scene to instantiate inside the scroll container.
@export var content_scene: PackedScene
## How much of the screen the card occupies in HALF state (0.0 = bottom, 1.0 = full).
@export_range(0.0, 1.0) var half_height_ratio: float = 0.5
## If true, close button hides the sheet completely and emits close_requested.
## If false (default), close button returns to HALF state.
@export var closeable: bool = false

var drag_state := DragState.HALF
var dragging := false
var start_pos: Vector2
var drag_tween: Tween
var _card_half_position: float
var _tween_callback: Callable
var _tween_header_visible: bool
var _content_instance: Node

@onready var panel_container_header: PanelContainer = %PanelContainer_Header
@onready var panel_container_card: PanelContainer = %PanelContainer_Card
@onready var scroll_content: ScrollContainer = %ScrollContent
@onready var margin_container_show_more: MarginContainer = %MarginContainer_ShowMore


func _ready() -> void:
	mouse_filter = Control.MOUSE_FILTER_PASS

	if content_scene:
		_content_instance = content_scene.instantiate()
		scroll_content.add_child(_content_instance)

	panel_container_header.self_modulate = Color.TRANSPARENT
	panel_container_header.hide()

	panel_container_card.set_anchors_and_offsets_preset.call_deferred(Control.PRESET_FULL_RECT)
	panel_container_card.set_position.call_deferred(Vector2(0, _get_hidden_position()))
	_initialize_card_position.call_deferred()


func _initialize_card_position() -> void:
	panel_container_card.position.y = _get_hidden_position()
	_card_half_position = _get_half_position()
	tween_to(_card_half_position)


func get_content_instance() -> Node:
	return _content_instance


func _input(event: InputEvent) -> void:
	if not visible:
		return

	if event is InputEventScreenTouch:
		if event.pressed:
			start_pos = event.position
			dragging = true
		elif dragging:
			dragging = false
			var drag_distance: float = event.position.y - start_pos.y
			var gesture := DragGesture.IDLE

			if drag_distance > _DRAG_THRESHOLD:
				gesture = DragGesture.DOWN
			elif drag_distance < -_DRAG_THRESHOLD:
				gesture = DragGesture.UP

			if _is_scrolling(event.position, gesture):
				return

			match gesture:
				DragGesture.UP:
					match drag_state:
						DragState.PEEK:
							drag_state = DragState.HALF
							tween_to(_card_half_position)
						DragState.HALF:
							drag_state = DragState.FULL
							tween_to(0.0, Callable(), true)
				DragGesture.DOWN:
					match drag_state:
						DragState.FULL:
							panel_container_header.hide()
							panel_container_card.position.y += panel_container_header.size.y
							_set_card_corner_radius(24, 24)
							drag_state = DragState.HALF
							tween_to(_card_half_position)
						DragState.HALF:
							drag_state = DragState.PEEK
							tween_to(_get_peek_position())


func _on_button_show_more_pressed() -> void:
	if drag_state == DragState.PEEK:
		drag_state = DragState.HALF
		tween_to(_card_half_position)
	elif drag_state == DragState.HALF:
		drag_state = DragState.FULL
		tween_to(0.0, Callable(), true)


func _on_button_close_pressed() -> void:
	if closeable:
		tween_to(_get_hidden_position(), close_requested.emit)
	else:
		if drag_state == DragState.FULL:
			panel_container_header.hide()
			panel_container_card.position.y += panel_container_header.size.y
			_set_card_corner_radius(24, 24)
		drag_state = DragState.HALF
		tween_to(_card_half_position)


func _is_scrolling(pos: Vector2, gesture: DragGesture) -> bool:
	var scroll_rect := scroll_content.get_global_rect()
	if not scroll_rect.has_point(pos):
		return false
	if gesture == DragGesture.DOWN and scroll_content.get_v_scroll() > _DRAG_THRESHOLD:
		return true
	return false


func _get_half_position() -> float:
	return get_rect().size.y * (1.0 - half_height_ratio)


func _get_peek_position() -> float:
	return get_rect().size.y - _PEEK_HEIGHT


func _get_hidden_position() -> float:
	return get_rect().size.y


func tween_to(
	y_position: float, callback: Callable = Callable(), header_visible: bool = false
) -> void:
	if drag_tween and drag_tween.is_running():
		drag_tween.stop()
		drag_tween = null
	_tween_callback = callback if callback.is_valid() else Callable()
	_tween_header_visible = header_visible

	if header_visible:
		panel_container_header.show()
		panel_container_header.self_modulate = Color.TRANSPARENT
		panel_container_card.position.y -= panel_container_header.size.y
		_set_card_corner_radius(0, 0)
	if not header_visible:
		margin_container_show_more.show()
		margin_container_show_more.self_modulate = Color.TRANSPARENT

	drag_tween = create_tween().set_trans(Tween.TRANS_QUART)
	drag_tween.set_parallel(true)
	drag_tween.tween_property(panel_container_card, "position:y", y_position, _TWEEN_DURATION)
	var header_target := Color.WHITE if header_visible else Color.TRANSPARENT
	drag_tween.tween_property(
		panel_container_header, "self_modulate", header_target, _TWEEN_DURATION
	)
	var show_more_target := Color.TRANSPARENT if header_visible else Color.WHITE
	drag_tween.tween_property(
		margin_container_show_more, "self_modulate", show_more_target, _TWEEN_DURATION
	)
	drag_tween.set_parallel(false)
	drag_tween.tween_callback(_on_tween_to_finished)


func _on_tween_to_finished() -> void:
	if not _tween_header_visible:
		panel_container_header.hide()
	if _tween_header_visible:
		margin_container_show_more.hide()
	if _tween_callback.is_valid():
		_tween_callback.call()


func _set_card_corner_radius(top_left: int, top_right: int) -> void:
	var current_style = panel_container_card.get_theme_stylebox("panel")
	if current_style is StyleBoxFlat:
		var style_box := current_style.duplicate() as StyleBoxFlat
		style_box.corner_radius_top_left = top_left
		style_box.corner_radius_top_right = top_right
		panel_container_card.add_theme_stylebox_override("panel", style_box)
