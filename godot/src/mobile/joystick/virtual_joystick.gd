@tool
class_name VirtualJoystick

extends Control
signal stick_position(Vector2)
signal is_holded(bool)
## A simple virtual joystick for touchscreens, with useful options.
## Github: https://github.com/MarcoFazioRandom/Virtual-Joystick-Godot

# EXPORTED VARIABLE

## The joystick doesn't move.
## Every time the joystick area is pressed,
## the joystick position is set on the touched position.
enum JoystickMode { FIXED, DYNAMIC }

## Always visible
## Visible on touch screens only
enum VisibilityMode { ALWAYS, TOUCHSCREEN_ONLY }

# Touch indices coming from gui_input are >= 0; this sentinel marks a gesture
# handed off from scene UI (finger started on a UI element then swiped into the
# joystick zone) so it never collides with a real gui touch.
const EXTERNAL_TOUCH_INDEX: int = -2

## If the input is inside this range, the output is zero.
@export_range(0, 200, 1) var deadzone_size: float = 0

## The max distance the tip can reach.
@export_range(0, 500, 1) var clampzone_size: float = 75

## Joystick resting position
@export var _joystick_default_position := Vector2.ZERO

## If the joystick stays in the same position or appears on the touched
## position when touch is started
@export var joystick_mode := JoystickMode.FIXED

## If the joystick is always visible, or is shown only if there is a touchscreen
@export var visibility_mode := VisibilityMode.ALWAYS

## If true, the joystick uses Input Actions (Project -> Project Settings -> Input Map)
@export var use_input_actions := true

@export var action_left := "ia_left"
@export var action_right := "ia_right"
@export var action_up := "ia_forward"
@export var action_down := "ia_backward"
@export var action_walk := "ia_walk"
@export var action_sprint := "ia_sprint"

## Shared HUD safe-area inset profile (assign hud_margins.tres). Places the resting joystick
## relative to the safe area. A Resource ref (unlike a node path) survives editor re-saves.
@export var margin_profile: SafeAreaMargins

# PUBLIC VARIABLES

## If the joystick is receiving inputs.
var is_pressed := false

# The joystick output.
var output := Vector2.ZERO

# PRIVATE VARIABLES

var touch_index: int = -1
var _joystick_position := Vector2.ZERO
var _tip_position := Vector2.ZERO
var _joystick_visible := false

@onready var _sprint_timer := %SprintTimer

@onready var _dynamic_material: ShaderMaterial = $Dynamic.material
@onready var _active_area: Control = $ActiveArea

@onready var _tip_default_position := Vector2.ZERO

# FUNCTIONS


func _ready() -> void:
	# _process only drives the editor preview; at runtime it has nothing to do per frame.
	set_process(Engine.is_editor_hint())
	if Engine.is_editor_hint():
		# Editor preview: draw the resting base at its computed position (no touch/timers/Global).
		_dynamic_material.set_shader_parameter("state", 0)
		_reset()
		return

	_sprint_timer.timeout.connect(func(): Input.action_press(action_sprint))

	if (
		not DisplayServer.is_touchscreen_available()
		and visibility_mode == VisibilityMode.TOUCHSCREEN_ONLY
	):
		hide()

	_active_area.gui_input.connect(_on_gui_input)

	Global.loading_started.connect(_on_loading_scene)
	_on_loading_scene()


func _on_loading_scene() -> void:
	_dynamic_material.set_shader_parameter("state", 0)


## Hides the joystick graphic + touch area. Used when a scene hides the native
## joystick via PBTouchscreenInputControls.
func set_visuals_hidden(hidden: bool) -> void:
	$Dynamic.visible = not hidden
	_active_area.visible = not hidden


func _on_gui_input(event: InputEvent) -> void:
	if not Global.is_mobile():
		return
	if event is InputEventScreenTouch:
		if event.pressed:
			# Pressing the joystick zone must hand keyboard/GUI focus back to the
			# explorer. Player movement is gated on ui_root having focus
			# (player.gd zeroes input_dir when not explorer_has_focus), but any
			# panel we came from (e.g. the profile passport's buttons) leaves focus
			# on itself. Because the joystick consumes its own touches, they never
			# reach mobile_camera_input, which is the only other place that regrabs
			# focus — so without this the joystick animates but the avatar won't
			# move after dismissing such a panel (#2361).
			if not Global.explorer_has_focus():
				Global.explorer_grab_focus()
			if touch_index != -1:
				return
			if joystick_mode == JoystickMode.FIXED and not _is_point_inside_base(event.position):
				return
			if joystick_mode == JoystickMode.DYNAMIC:
				_move_base(event.position)
				get_tree().create_timer(0.25).timeout.connect(_on_show_joystick_timer)
			touch_index = event.index
			_update_joystick(event.position)
			_consume_if_not_cinematic()
		elif event.index == touch_index:
			_reset()
			if _joystick_visible:
				_dynamic_material.set_shader_parameter("state", 2)
				_joystick_visible = false
			emit_signal("stick_position", Vector2.ZERO)
			_consume_if_not_cinematic()
	elif event is InputEventScreenDrag:
		if event.index == touch_index:
			_update_joystick(event.position)
			_consume_if_not_cinematic()


func _consume_if_not_cinematic() -> void:
	# Cinematic camera mode lets the event bubble to ui_root.gui_input so
	# explorer.gd can fire ia_pointer for scene click handlers.
	if not Global.scene_runner.raycast_use_cursor_position:
		_active_area.accept_event()


## Begin a joystick gesture from an arbitrary screen point. Used when a scene-UI
## press is swiped out into the joystick zone: the base is placed at the original
## touch position so movement keys engage immediately.
func external_begin(start_position: Vector2) -> void:
	if touch_index != -1:
		return
	touch_index = EXTERNAL_TOUCH_INDEX
	_move_base(start_position)
	_dynamic_material.set_shader_parameter("state", 1)
	_joystick_visible = true
	_update_joystick(start_position)


func external_update(position: Vector2) -> void:
	if touch_index != EXTERNAL_TOUCH_INDEX:
		return
	_update_joystick(position)


func external_end() -> void:
	if touch_index != EXTERNAL_TOUCH_INDEX:
		return
	_reset()
	if _joystick_visible:
		_dynamic_material.set_shader_parameter("state", 2)
		_joystick_visible = false
	emit_signal("stick_position", Vector2.ZERO)


func get_active_area_global_rect() -> Rect2:
	return _active_area.get_global_rect()


func _on_show_joystick_timer() -> void:
	if touch_index != -1:
		_dynamic_material.set_shader_parameter("state", 1)
		_joystick_visible = true


func _move_base(new_position: Vector2) -> void:
	_joystick_position = new_position
	_dynamic_material.set_shader_parameter("joystick_position", _joystick_position)


func _move_tip(vector: Vector2) -> void:
	_dynamic_material.set_shader_parameter("tip_position", vector)


func _is_point_inside_base(point: Vector2) -> bool:
	var center: Vector2 = _joystick_position
	var vector: Vector2 = point - center
	if vector.length_squared() <= 25.0 * 25.0:
		return true

	return false


func _update_joystick(touch_position: Vector2) -> void:
	var center: Vector2 = _joystick_position
	var vector: Vector2 = touch_position - center

	_move_tip(vector)

	if vector.length_squared() > deadzone_size * deadzone_size:
		is_pressed = true
		output = (vector - (vector.normalized() * deadzone_size)) / (clampzone_size - deadzone_size)
	else:
		is_pressed = false
		output = Vector2.ZERO

	if use_input_actions:
		_update_input_actions()
	else:
		emit_signal("stick_position", output)


func _update_input_actions():
	if output.x < 0:
		Input.action_press(action_left, -output.x)
	elif Input.is_action_pressed(action_left):
		Input.action_release(action_left)
	if output.x > 0:
		Input.action_press(action_right, output.x)
	elif Input.is_action_pressed(action_right):
		Input.action_release(action_right)
	if output.y < 0:
		Input.action_press(action_up, -output.y)
	elif Input.is_action_pressed(action_up):
		Input.action_release(action_up)
	if output.y > 0:
		Input.action_press(action_down, output.y)
	elif Input.is_action_pressed(action_down):
		Input.action_release(action_down)
	if output.length() < 0.95:
		Input.action_release(action_sprint)
		_sprint_timer.stop()
	elif _sprint_timer.is_stopped() and !Input.is_action_pressed(action_sprint):
		_sprint_timer.start()


## Editor-only: keep the resting preview in sync with the mobile-preview safe area and the
## _joystick_default_position value (SafeAreaControls recomputes its insets every frame too).
func _process(_delta: float) -> void:
	# Enabled only in-editor (see set_process in _ready): keep the preview synced to size/margin edits.
	_reset()


func _reset():
	is_pressed = false
	if not Engine.is_editor_hint():
		emit_signal("is_holded", false)
	output = Vector2.ZERO
	touch_index = -1

	# `_joystick_default_position` is relative to the SAFE AREA (x from the left edge, y from the
	# bottom). Offset it by the safe-area inset so the resting spot adapts per device. Uses the shared
	# HUD margin PROFILE (108/45 landscape; same one the joypad/chat frame use) floored against the
	# live device safe-area inset. A Resource @export (not a node-path) is used because node-path
	# exports get dropped from the .tscn when the editor re-saves an instanced scene.
	var safe_left := 0.0
	var safe_bottom := 0.0
	if margin_profile:
		var portrait: bool = (not Engine.is_editor_hint()) and Global.is_orientation_portrait()
		safe_left = float(margin_profile.portrait_left if portrait else margin_profile.left)
		safe_bottom = float(margin_profile.portrait_bottom if portrait else margin_profile.bottom)
	if not Engine.is_editor_hint() and (Global.is_mobile() or Global.is_emulating_safe_area()):
		var sa: Rect2i = Global.get_safe_area()
		var win: Vector2i = DisplayServer.window_get_size()
		var vp: Vector2 = get_viewport().get_visible_rect().size
		safe_left = maxf(safe_left, float(sa.position.x) * (vp.x / float(win.x)))
		safe_bottom = maxf(safe_bottom, float(abs(sa.end.y - win.y)) * (vp.y / float(win.y)))
	var base_position := Vector2(
		_joystick_default_position.x + safe_left,
		size.y - _joystick_default_position.y - safe_bottom
	)

	_dynamic_material.set_shader_parameter("max_chain_length", size.length() * 0.25)

	_move_base(base_position)

	_tip_position = _tip_default_position
	_move_tip(_tip_position)

	if use_input_actions and not Engine.is_editor_hint():
		if Input.is_action_pressed(action_left) or Input.is_action_just_pressed(action_left):
			Input.action_release(action_left)
		if Input.is_action_pressed(action_right) or Input.is_action_just_pressed(action_right):
			Input.action_release(action_right)
		if Input.is_action_pressed(action_down) or Input.is_action_just_pressed(action_down):
			Input.action_release(action_down)
		if Input.is_action_pressed(action_up) or Input.is_action_just_pressed(action_up):
			Input.action_release(action_up)
		if Input.is_action_pressed(action_walk) or Input.is_action_just_pressed(action_walk):
			Input.action_release(action_walk)
		if Input.is_action_pressed(action_sprint) or Input.is_action_just_pressed(action_sprint):
			Input.action_release(action_sprint)
		_sprint_timer.stop()


func _on_resized() -> void:
	if not is_node_ready():
		return
	_reset()
