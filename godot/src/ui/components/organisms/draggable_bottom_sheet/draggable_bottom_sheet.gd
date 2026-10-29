class_name DraggableBottomSheet
extends VBoxContainer

## Generic draggable bottom sheet with three states: PEEK, HALF, FULL.
## Swipe up: PEEK → HALF → FULL. Swipe down: FULL → HALF → PEEK.
## PEEK always shows a small grab area so the user can drag back up.
## Set content_scene to inject a scene, half_height_ratio for HALF position,
## and closeable to control whether the close button hides or returns to HALF.

signal close_requested
signal drag_state_changed(new_state: DragState)

enum DragState { PEEK, HALF, FULL, HIDDEN }
enum DragGesture { IDLE, UP, DOWN }

const _TWEEN_DURATION := 0.2
const _DRAG_THRESHOLD := 50.0
const _PEEK_HEIGHT := 64.0

## Scene to instantiate inside the scroll container.
@export var content_scene: PackedScene
## How much of the screen the card occupies in HALF state (0.0 = bottom, 1.0 = full).
@export_range(0.0, 1.0) var half_height_ratio: float = 0.5
## If true, close button hides the sheet completely and emits close_requested.
## If false (default), close button returns to HALF state.
@export var closeable: bool = false
## When false, FULL never shows the header/close bar — used when something else (e.g. a
## page's own top tabs) stays on screen regardless, so the sheet never needs an explicit
## close affordance; the drag-down gesture is still the only way to collapse it.
@export var show_close_button: bool = true
## Card/header background color. Defaults to Profile's own card color, so Profile needs no
## changes; a host whose content has a differently-toned background (e.g. Backpack's
## wearables gradient) can override this to match it, so the strip above the content —
## where the drag handle sits — doesn't show a seam against the content's own color.
@export var card_color: Color = Color(0.20784314, 0.03137255, 0.32941177, 1)
## Padding around the drag handle, above and below it. Defaults to Profile's own spacing,
## so Profile needs no changes; a host that wants a tighter handle zone can override these.
@export var handle_margin_top: int = 16
@export var handle_margin_bottom: int = 40
## The scroll container _is_scrolling() checks to tell "the user is scrolling the content"
## apart from "the user is dragging the sheet". Defaults to the sheet's own %ScrollContent
## (Profile's case: its content sits directly inside it). A host whose real scrollable
## content lives deeper — e.g. Backpack's content_scene has its own inner grid scroll and
## leaves %ScrollContent's own scrolling disabled — overrides this to point at that inner
## ScrollContainer instead; otherwise this always reads 0 and every downward drag anywhere
## on the card collapses the sheet instead of scrolling the content.
@export var scroll_reference: ScrollContainer
## When true, a drag gesture only starts if the touch begins on the card/header — needed when
## something else above the card (e.g. Backpack's AvatarPreview, which rotates/pinch-zooms the
## camera there) also listens for raw touch input and would otherwise fight over the same
## events. Defaults to false (Profile's current behavior: swipe from anywhere), since Profile's
## portrait sheet has nothing above the card competing for touch input.
@export var restrict_drag_to_card: bool = false

var drag_state := DragState.HALF
var dragging := false
var start_pos: Vector2
var drag_tween: Tween
var _card_half_position: float
var _tween_callback: Callable
var _tween_header_visible: bool
var _tween_target_hidden: bool
var _content_instance: Node
var _ignore_input: bool = false
var _focus_out_state: DragState = DragState.HALF
var _state_at_touch_start: DragState = DragState.HALF
var _last_touch_end_ms: int = 0
var _active_touch_index: int = -1
var _scroll_value_at_touch_start: float = 0.0

@onready var panel_container_header: PanelContainer = %PanelContainer_Header
@onready var panel_container_card: PanelContainer = %PanelContainer_Card
@onready var scroll_content: ScrollContainer = %ScrollContent
@onready var margin_container_show_more: MarginContainer = %MarginContainer_ShowMore


func _ready() -> void:
	mouse_filter = Control.MOUSE_FILTER_PASS

	if content_scene:
		_content_instance = content_scene.instantiate()
		scroll_content.add_child(_content_instance)

	_apply_card_color(panel_container_header)
	_apply_card_color(panel_container_card)
	margin_container_show_more.add_theme_constant_override("margin_top", handle_margin_top)
	margin_container_show_more.add_theme_constant_override("margin_bottom", handle_margin_bottom)
	panel_container_header.self_modulate = Color.TRANSPARENT
	panel_container_header.hide()

	panel_container_card.set_position.call_deferred(Vector2(0, _get_hidden_position()))
	_initialize_card_position.call_deferred()
	resized.connect(_on_resized)


func _on_resized() -> void:
	if drag_tween and drag_tween.is_running():
		drag_tween.stop()
		drag_tween = null
	_card_half_position = _get_half_position()
	panel_container_card.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	match drag_state:
		DragState.PEEK:
			panel_container_card.position.y = _get_peek_position()
		DragState.HALF:
			panel_container_card.position.y = _card_half_position
		DragState.FULL:
			panel_container_card.position.y = 0.0


func _initialize_card_position() -> void:
	_card_half_position = _get_half_position()
	if not is_visible_in_tree():
		# Skip tween when hidden; reset_to_half() will handle positioning when shown.
		panel_container_card.position.y = _card_half_position
		return
	panel_container_card.position.y = _get_hidden_position()
	tween_to(_card_half_position)


func get_content_instance() -> Node:
	return _content_instance


func reset_to_half() -> void:
	drag_state = DragState.HALF
	panel_container_header.hide()
	_set_card_corner_radius(24, 24)
	panel_container_card.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	_card_half_position = _get_half_position()
	scroll_content.scroll_vertical = 0
	# drag_state_changed fires once the tween lands (_on_tween_to_finished) — not here, since
	# the card hasn't actually moved yet at this point (see tween_to()'s header-compensation
	# jump: a listener reacting to this position reads a transient, not-yet-settled value).
	tween_to(_card_half_position)


func _notification(what: int) -> void:
	if what == NOTIFICATION_APPLICATION_FOCUS_OUT:
		dragging = false
		_active_touch_index = -1
		# Block input immediately so phantom touch-cancel events (delivered by iOS
		# when the app backgrounds) cannot accidentally change the drag state.
		_ignore_input = true
		# On iOS (no home button), the swipe-up-to-home gesture delivers pressed=true
		# then pressed=false to the app and fully completes BEFORE FOCUS_OUT fires.
		# By the time we reach here, drag_state may already reflect the gesture's
		# outcome (e.g. HALF→FULL). We detect this by checking whether a touch ended
		# very recently: if so, we treat the pre-touch state as the authoritative
		# state the user left the card in.
		var ms_since_touch_end: int = Time.get_ticks_msec() - _last_touch_end_ms
		if _last_touch_end_ms > 0 and ms_since_touch_end < 300:
			_focus_out_state = _state_at_touch_start
		else:
			_focus_out_state = drag_state
	elif what == NOTIFICATION_APPLICATION_FOCUS_IN:
		# If the state changed between FOCUS_OUT and now it was driven by the
		# swipe-up-to-home gesture (or a phantom touch that slipped through before
		# FOCUS_OUT). Snap back to the state the user deliberately left the card in.
		if drag_state != _focus_out_state:
			_snap_to_state(_focus_out_state)
		# Keep input blocked for a short window after FOCUS_IN. On iOS, phantom
		# touches can also arrive between applicationWillEnterForeground (before
		# FOCUS_IN) and applicationDidBecomeActive (FOCUS_IN), so holding the block
		# past FOCUS_IN for 100 ms covers that gap too.
		get_tree().create_timer(0.1).timeout.connect(_enable_input, CONNECT_ONE_SHOT)


func _enable_input() -> void:
	_ignore_input = false


func _snap_to_state(state: DragState) -> void:
	if drag_tween and drag_tween.is_running():
		drag_tween.stop()
		drag_tween = null
	drag_state = state
	panel_container_card.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	match state:
		DragState.PEEK:
			panel_container_header.hide()
			_set_card_corner_radius(24, 24)
			margin_container_show_more.show()
			margin_container_show_more.self_modulate = Color.WHITE
			panel_container_card.position.y = _get_peek_position()
		DragState.HALF:
			panel_container_header.hide()
			_set_card_corner_radius(24, 24)
			margin_container_show_more.show()
			margin_container_show_more.self_modulate = Color.WHITE
			panel_container_card.position.y = _card_half_position
		DragState.FULL:
			if show_close_button:
				panel_container_header.show()
				panel_container_header.self_modulate = Color.WHITE
			_set_card_corner_radius(0, 0)
			margin_container_show_more.hide()
			panel_container_card.position.y = 0.0
		DragState.HIDDEN:
			panel_container_header.hide()
			margin_container_show_more.show()
			margin_container_show_more.self_modulate = Color.WHITE
			panel_container_card.position.y = _get_hidden_position()
	drag_state_changed.emit(state)


func _input(event: InputEvent) -> void:
	if not visible:
		return
	if _ignore_input:
		return

	if event is InputEventScreenTouch:
		if event.pressed:
			if _active_touch_index != -1:
				return
			if restrict_drag_to_card and not _is_on_card(event.position):
				return
			_active_touch_index = event.index
			start_pos = event.position
			dragging = true
			_state_at_touch_start = drag_state
			var scroller: ScrollContainer = scroll_reference if scroll_reference else scroll_content
			_scroll_value_at_touch_start = scroller.get_v_scroll()
		elif dragging and event.index == _active_touch_index:
			_active_touch_index = -1
			dragging = false
			_last_touch_end_ms = Time.get_ticks_msec()
			var drag_distance: float = event.position.y - start_pos.y
			var drag_distance_x: float = absf(event.position.x - start_pos.x)
			var gesture := DragGesture.IDLE

			if drag_distance_x > absf(drag_distance):
				gesture = DragGesture.IDLE
			elif drag_distance > _DRAG_THRESHOLD:
				gesture = DragGesture.DOWN
			elif drag_distance < -_DRAG_THRESHOLD:
				gesture = DragGesture.UP

			if _is_scrolling(event.position, gesture):
				return

			# drag_state_changed fires once each of these tweens lands (_on_tween_to_finished),
			# not here — a listener that re-fits against the card's position (e.g. Backpack's
			# avatar camera) needs the settled position, not the one at the start of the
			# animation (the header-compensation jump below is a transient, not a rest state).
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
		tween_to(_get_hidden_position(), close_requested.emit, false, true)
	else:
		if drag_state == DragState.FULL:
			panel_container_header.hide()
			panel_container_card.position.y += panel_container_header.size.y
			_set_card_corner_radius(24, 24)
		drag_state = DragState.HALF
		tween_to(_card_half_position)


func _is_on_card(pos: Vector2) -> bool:
	if panel_container_card.get_global_rect().has_point(pos):
		return true
	return (
		panel_container_header.visible and panel_container_header.get_global_rect().has_point(pos)
	)


func _is_scrolling(pos: Vector2, gesture: DragGesture) -> bool:
	var scroller: ScrollContainer = scroll_reference if scroll_reference else scroll_content
	var scroll_rect := scroller.get_global_rect()
	if not scroll_rect.has_point(pos):
		return false
	var v_scroll := scroller.get_v_scroll()
	if not scroll_reference:
		# Profile's own %ScrollContent: unchanged from the original, DOWN-only threshold
		# check. Gating the richer logic below behind an explicit scroll_reference keeps
		# Profile's swipe-up-to-expand working exactly as before — it never sets one.
		return gesture == DragGesture.DOWN and v_scroll > _DRAG_THRESHOLD
	# The content actually moved during this touch — it was a scroll, not a sheet-drag,
	# regardless of how close the release landed to either edge. A fixed "near the edge"
	# threshold here misfired on a slow scroll that ended within it (e.g. browsing items one
	# by one near the bottom of the list) even though the whole gesture was clearly a scroll.
	if v_scroll != _scroll_value_at_touch_start:
		return true
	if gesture == DragGesture.DOWN:
		# Still scrolled down from the top — there's room to keep scrolling toward it;
		# don't let this collapse the sheet.
		return v_scroll > 0.0
	if gesture == DragGesture.UP:
		# Not yet at the bottom of the content — there's more to reveal by scrolling;
		# don't let this expand the sheet instead.
		var v_scroll_bar := scroller.get_v_scroll_bar()
		var max_scroll: float = v_scroll_bar.max_value - v_scroll_bar.page
		return v_scroll < max_scroll
	return false


func _get_half_position() -> float:
	return get_rect().size.y * (1.0 - half_height_ratio)


func _get_peek_position() -> float:
	return get_rect().size.y - _PEEK_HEIGHT


func _get_hidden_position() -> float:
	return get_rect().size.y


func tween_to(
	y_position: float,
	callback: Callable = Callable(),
	header_visible: bool = false,
	target_hidden: bool = false
) -> void:
	header_visible = header_visible and show_close_button
	if drag_tween and drag_tween.is_running():
		drag_tween.stop()
		drag_tween = null
	_tween_callback = callback if callback.is_valid() else Callable()
	_tween_header_visible = header_visible
	_tween_target_hidden = target_hidden

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
	if _tween_target_hidden:
		drag_state = DragState.HIDDEN
	# Emitted once the card has actually settled, not when the tween starts — a listener
	# that re-fits content against the card's position (e.g. Backpack's avatar camera)
	# needs the final position, not a transient one mid-animation.
	drag_state_changed.emit(drag_state)
	if _tween_callback.is_valid():
		_tween_callback.call()


## Duplicates so later style overrides (_set_card_corner_radius) build on this instance's
## color instead of resetting to the shared base resource's.
func _apply_card_color(panel: PanelContainer) -> void:
	var current_style = panel.get_theme_stylebox("panel")
	if current_style is StyleBoxFlat:
		var style_box := current_style.duplicate() as StyleBoxFlat
		style_box.bg_color = card_color
		panel.add_theme_stylebox_override("panel", style_box)


func _set_card_corner_radius(top_left: int, top_right: int) -> void:
	var current_style = panel_container_card.get_theme_stylebox("panel")
	if current_style is StyleBoxFlat:
		var style_box := current_style.duplicate() as StyleBoxFlat
		style_box.corner_radius_top_left = top_left
		style_box.corner_radius_top_right = top_right
		var is_full := top_left == 0 and top_right == 0
		# anti_aliasing on a StyleBoxFlat with no border draws a stray light seam along the
		# rounded edge (no border color for the AA blend to land on) — leave it off.
		style_box.anti_aliasing = false
		style_box.expand_margin_bottom = 4 if is_full else 0
		panel_container_card.add_theme_stylebox_override("panel", style_box)
