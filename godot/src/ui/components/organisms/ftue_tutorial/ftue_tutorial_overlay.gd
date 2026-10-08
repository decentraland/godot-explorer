class_name FtueTutorialOverlay
extends Control

## View of the guided tutorial (issue #2767): dim with a cut-out, highlight ring, tooltip,
## gesture icon and the skip button. FtueTutorialRunner decides what each step shows.

signal skip_pressed

enum Place { ABOVE, BELOW, RIGHT, LEFT, CENTER }
enum Gesture { NONE, DRAG, PINCH }

const TOOLTIP_GAP := 18.0
const DRAG_HOLD_SECONDS := 1.5
const PINCH_HOLD_SECONDS := 0.8
const DISSOLVE_SECONDS := 0.3
const DRAG_OFFSET := Vector2(-60.0, 24.0)
const GESTURE_ALPHA := 0.8
const MOUSE_INDEX := -1

# Touches outside this rect are swallowed while `_blocking` is on.
var _allowed_rect := Rect2()
var _blocking := false
# Presses swallowed here, so their drags and releases are swallowed too.
var _owned_touches: Dictionary = {}

# Distance from the screen edges that is clear of notches and rounded corners.
var _edge_inset := Vector2(TOOLTIP_GAP, TOOLTIP_GAP)
var _tooltip_target := Rect2()
var _tooltip_place := Place.ABOVE
var _gesture_tween: Tween

@onready var dim: ColorRect = %Dim
@onready var ring: FtueHighlightRing = %Ring
@onready var tooltip: ControlsFtueLabel = %Tooltip
@onready var tooltip_label: Label = %Label_Tooltip
@onready var icon_a: TextureRect = %Icon_A
@onready var icon_b: TextureRect = %Icon_B
@onready var button_skip: Button = %Button_Skip


func _ready() -> void:
	dim.hide()
	ring.hide()
	tooltip.hide()
	icon_a.hide()
	icon_b.hide()
	tooltip.resized.connect(_place_tooltip)
	resized.connect(_on_resized)
	_on_resized()


func show_dim(hole: Rect2, corner_radius: float) -> void:
	var material := dim.material as ShaderMaterial
	material.set_shader_parameter(
		"hole", Vector4(hole.position.x, hole.position.y, hole.size.x, hole.size.y)
	)
	material.set_shader_parameter(
		"radius", minf(corner_radius, minf(hole.size.x, hole.size.y) * 0.5)
	)
	dim.show()


func hide_dim() -> void:
	dim.hide()


## While blocking, only touches inside `allowed` (and on the skip button) reach the app.
func block_input_except(allowed: Rect2) -> void:
	_blocking = true
	_allowed_rect = allowed


func free_input() -> void:
	_blocking = false


func show_tooltip(key: String, target: Rect2, place: Place) -> void:
	tooltip_label.text = key
	_tooltip_target = target
	_tooltip_place = place
	tooltip.show()
	tooltip.reset_size()
	_place_tooltip.call_deferred()


func hide_tooltip() -> void:
	tooltip.hide()


func show_gesture(gesture: Gesture, center: Vector2) -> void:
	hide_gesture()
	if gesture == Gesture.NONE:
		return
	icon_a.show()
	icon_a.modulate.a = GESTURE_ALPHA
	_gesture_tween = create_tween().set_loops()
	if gesture == Gesture.DRAG:
		# One hand sliding right to left and slightly down, then back.
		icon_a.texture = preload("assets/hand_drag.png")
		var from := center - icon_a.texture.get_size() * 0.5
		icon_a.position = from
		_gesture_tween.tween_interval(DRAG_HOLD_SECONDS)
		_gesture_tween.tween_property(icon_a, "position", from + DRAG_OFFSET, DISSOLVE_SECONDS)
		_gesture_tween.tween_interval(DRAG_HOLD_SECONDS)
		_gesture_tween.tween_property(icon_a, "position", from, DISSOLVE_SECONDS)
		return
	# Two icons cross-fading: fingers apart, fingers together.
	icon_a.texture = preload("assets/pinch_out.png")
	icon_b.texture = preload("assets/pinch_in.png")
	icon_a.position = center - icon_a.texture.get_size() * 0.5
	icon_b.position = center - icon_b.texture.get_size() * 0.5
	icon_b.show()
	icon_b.modulate.a = 0.0
	_gesture_tween.tween_interval(PINCH_HOLD_SECONDS)
	_gesture_tween.tween_property(icon_a, "modulate:a", 0.0, DISSOLVE_SECONDS)
	_gesture_tween.parallel().tween_property(icon_b, "modulate:a", GESTURE_ALPHA, DISSOLVE_SECONDS)
	_gesture_tween.tween_interval(PINCH_HOLD_SECONDS)
	_gesture_tween.tween_property(icon_a, "modulate:a", GESTURE_ALPHA, DISSOLVE_SECONDS)
	_gesture_tween.parallel().tween_property(icon_b, "modulate:a", 0.0, DISSOLVE_SECONDS)


func hide_gesture() -> void:
	if _gesture_tween != null:
		_gesture_tween.kill()
		_gesture_tween = null
	icon_a.hide()
	icon_b.hide()


## Top-right, mirroring the menu button's inset from the top-left corner (which already clears
## the notch). `at_bottom_left` is for screens whose top-right corner is taken.
func place_skip_button(menu_button_rect: Rect2, at_bottom_left := false) -> void:
	_edge_inset = menu_button_rect.position
	button_skip.reset_size()
	if at_bottom_left:
		button_skip.global_position = Vector2(
			_edge_inset.x, size.y - _edge_inset.y - button_skip.size.y
		)
	else:
		button_skip.global_position = Vector2(
			size.x - _edge_inset.x - button_skip.size.x, _edge_inset.y
		)


# _input, not _gui_input: MobileCameraInput recognizes gestures in _input, before any Control
# could stop them, and this node runs first because it sits later in the tree.
func _input(event: InputEvent) -> void:
	var index := MOUSE_INDEX
	var pressed := false
	var position := Vector2.ZERO
	if event is InputEventScreenTouch:
		index = event.index
		pressed = event.pressed
		position = event.position
	elif event is InputEventMouseButton:
		pressed = event.pressed
		position = event.position
	elif event is InputEventScreenDrag:
		if _owned_touches.has(event.index):
			get_viewport().set_input_as_handled()
		return
	else:
		return

	if not pressed:
		if _owned_touches.erase(index):
			get_viewport().set_input_as_handled()
		return
	if not _blocking or not visible:
		return
	if _allowed_rect.has_point(position) or button_skip.get_global_rect().has_point(position):
		return
	_owned_touches[index] = true
	get_viewport().set_input_as_handled()


func _on_resized() -> void:
	(dim.material as ShaderMaterial).set_shader_parameter("rect_size", size)
	_place_tooltip()


func _place_tooltip() -> void:
	if not tooltip.visible:
		return
	var target := _tooltip_target
	var center := target.get_center()
	var tip_size := tooltip.size
	var where := Vector2.ZERO
	match _tooltip_place:
		Place.ABOVE:
			where = Vector2(
				center.x - tip_size.x * 0.5, target.position.y - TOOLTIP_GAP - tip_size.y
			)
		Place.BELOW:
			where = Vector2(target.position.x, target.end.y + TOOLTIP_GAP)
		Place.RIGHT:
			where = Vector2(target.end.x + TOOLTIP_GAP, center.y - tip_size.y * 0.5)
		Place.LEFT:
			where = Vector2(target.position.x - TOOLTIP_GAP - tip_size.x, target.position.y)
		Place.CENTER:
			where = center - tip_size * 0.5
	where.x = clampf(
		where.x, _edge_inset.x, maxf(_edge_inset.x, size.x - tip_size.x - _edge_inset.x)
	)
	tooltip.global_position = where


func _on_button_skip_pressed() -> void:
	skip_pressed.emit()
