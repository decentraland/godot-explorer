class_name MobileCameraInput
extends Control

# Gesture adopted from scene UI (finger pressed a UI element then swiped off it).
# Kept separate from the gui_input state below so it never collides with normal
# touches handled by this catcher.
enum AdoptedMode { NONE, CAMERA, JOYSTICK }

const HORIZONTAL_SENS: float = 0.5
const VERTICAL_SENS: float = 0.5

# --- Full-screen pinch recognizer (issue #2636 follow-up) --------------------
# Two-finger pinch is recognized GLOBALLY (from _input, which sees every touch
# regardless of which control is under it) so it works over the whole screen —
# including the joystick's dynamic active area (the whole left third), where the
# gui_input path never sees the finger because the joystick consumes it.
#
# To avoid stealing legitimate concurrent gestures, a pinch only COMMITS when all
# three hold for the two candidate fingers:
#   1. both fingers moved (rejects joystick-walk + button-TAP: the tap is static);
#   2. the finger spread crossed a threshold (ignores incidental jitter);
#   3. most of that motion is AXIAL-opposing — the fingers move along the line
#      between them, apart or together (rejects walk + look, whose two fingers
#      move in unrelated directions).
# Only on commit does it take over: it cancels the joystick (so the avatar stops
# the instant the pinch wins) and stops the single-finger look. Until then every
# touch flows normally, so walk / look / button taps are untouched.
const PINCH_MIN_FINGER_MOVE: float = 8.0
const PINCH_COMMIT_SPREAD: float = 20.0
const PINCH_AXIAL_FRACTION: float = 0.6
# A pinch finger that started over the joystick's active area must travel this far
# before it counts — past the joystick clampzone (~75px), where a walk tilt
# settles. Below it, the finger is treated as walking, not pinching.
const PINCH_JOYSTICK_MIN_TRAVEL: float = 90.0

var _player: Player = null
var _chat_panel: Control = null
var _joystick: VirtualJoystick = null

# Single-finger camera look. Driven from gui_input (free touches only), so a
# finger over the joystick / a button / scene UI never looks. Independent from the
# global pinch tracking below.
var _look_index: int = -1

# Every active touch (index → position), tracked in _input so pinch recognition
# is independent of what UI owns each finger.
var _touches: Dictionary = {}
var _pinch_active: bool = false
var _pinch_a: int = -1
var _pinch_b: int = -1
# Candidate start positions (seeded when the count reaches two) — the baseline the
# recognizer measures finger travel and axial spread against.
var _pinch_start_a: Vector2 = Vector2.ZERO
var _pinch_start_b: Vector2 = Vector2.ZERO
var _pinch_prev_distance: float = 0.0

# Adopted scene-UI swipe gestures, keyed by touch index → AdoptedMode. Keyed per
# finger so concurrent breakouts don't clobber each other's state.
var _adopted: Dictionary = {}


func _ready() -> void:
	if not Global.is_mobile():
		hide()
		set_process_input(false)
		return
	mouse_filter = MOUSE_FILTER_STOP
	gui_input.connect(_on_gui_input)
	_resolve_player.call_deferred()


func _resolve_player() -> void:
	var explorer := Global.get_explorer()
	if explorer:
		_player = explorer.player as Player
		_chat_panel = explorer.chat_panel
		_joystick = explorer.virtual_joystick as VirtualJoystick


# Backgrounding the app or an OS gesture-cancel can drop the touch-release events,
# leaving stale entries that would seed a bogus finger-spread on the next pinch.
# Clear all touch state on focus loss so the next gesture starts clean.
func _notification(what: int) -> void:
	if what == NOTIFICATION_APPLICATION_FOCUS_OUT:
		_touches.clear()
		_look_index = -1
		if _pinch_active:
			_end_pinch()


func _is_chat_visible() -> bool:
	if not is_instance_valid(_chat_panel):
		var explorer := Global.get_explorer()
		if explorer:
			_chat_panel = explorer.chat_panel
	return is_instance_valid(_chat_panel) and _chat_panel.is_chat_visible()


# Global touch tracking + pinch recognition. Runs before gui_input, so it can take
# over a finger the joystick/UI would otherwise own. Only pinch-owned touches are
# consumed here (set_input_as_handled); everything else falls through untouched.
func _input(event: InputEvent) -> void:
	# Safety net: an adopted scene-UI finger is drag-captured by its control, which
	# normally forwards the release; end the gesture here too in case that control
	# was freed/hidden mid-gesture before it could (otherwise the joystick could
	# stay engaged with no finger down).
	if event is InputEventScreenTouch and not event.pressed and _adopted.has(event.index):
		_end_adopted(event.index)

	if _player == null:
		return
	# No zoom in cinematic / pointer mode — gui_input drives the pointer there.
	if Global.scene_runner.raycast_use_cursor_position:
		return

	if event is InputEventScreenTouch:
		if event.pressed:
			_touches[event.index] = event.position
			_on_touch_count_changed()
		else:
			var was_pinch_finger: bool = (
				_pinch_active and (event.index == _pinch_a or event.index == _pinch_b)
			)
			_touches.erase(event.index)
			_on_touch_count_changed()
			# Swallow a pinch finger's release so gui_input doesn't read it as a look end.
			if was_pinch_finger:
				get_viewport().set_input_as_handled()
	elif event is InputEventScreenDrag:
		_touches[event.index] = event.position
		if _pinch_active:
			if event.index == _pinch_a or event.index == _pinch_b:
				_update_pinch()
				get_viewport().set_input_as_handled()
		elif _touches.size() >= 2:
			_try_recognize_pinch()
			if _pinch_active:
				get_viewport().set_input_as_handled()


# (Re)establish the candidate pair when the touch count changes; end the pinch
# when it drops below two.
func _on_touch_count_changed() -> void:
	if _touches.size() < 2:
		if _pinch_active:
			_end_pinch()
		return
	if not _pinch_active:
		_seed_pinch_candidate()
	elif not _touches.has(_pinch_a) or not _touches.has(_pinch_b):
		# A committed pinch finger lifted but two others remain: re-seat onto the
		# survivors so the gesture keeps going instead of stranding.
		_reseat_pinch()


func _seed_pinch_candidate() -> void:
	var indices: Array = _touches.keys()
	_pinch_a = indices[0]
	_pinch_b = indices[1]
	_pinch_start_a = _touches[_pinch_a]
	_pinch_start_b = _touches[_pinch_b]
	_pinch_prev_distance = _pinch_start_a.distance_to(_pinch_start_b)


func _try_recognize_pinch() -> void:
	if not _touches.has(_pinch_a) or not _touches.has(_pinch_b):
		return
	var a: Vector2 = _touches[_pinch_a]
	var b: Vector2 = _touches[_pinch_b]
	var move_a: float = a.distance_to(_pinch_start_a)
	var move_b: float = b.distance_to(_pinch_start_b)
	# 1. Each finger must move past its threshold. A static finger (a button tap)
	# is never a pinch. A finger that started over the joystick's active area must
	# travel PAST the joystick clampzone: walk-left + look-right is geometrically
	# a pinch-out (fingers spreading), and the one thing that separates a walk from
	# a pinch is reach — a walk tilt settles inside the clampzone (pushing further
	# does nothing), only a deliberate pinch spread exceeds it. This lets the pinch
	# work over the joystick (true full-screen) without walk+look hijacking a zoom.
	if move_a < _finger_pinch_threshold(_pinch_start_a):
		return
	if move_b < _finger_pinch_threshold(_pinch_start_b):
		return
	var axis: Vector2 = _pinch_start_b - _pinch_start_a
	if axis.length() < 1.0:
		return
	axis = axis.normalized()
	# Positive = fingers spreading apart along their axis, negative = closing.
	var axial_spread: float = (b - _pinch_start_b).dot(axis) - (a - _pinch_start_a).dot(axis)
	# 2. Spread must cross the threshold.
	if absf(axial_spread) < PINCH_COMMIT_SPREAD:
		return
	# 3. Most of the motion must be axial-opposing — rejects walk + look, whose
	# fingers move in unrelated directions (low axial fraction of total travel).
	if absf(axial_spread) < PINCH_AXIAL_FRACTION * (move_a + move_b):
		return
	_commit_pinch(a, b)


## Minimum travel for a finger to count toward a pinch. A finger that started over
## the joystick's active area must exceed the clampzone (a walk tilt settles inside
## it); anywhere else the small default applies.
func _finger_pinch_threshold(start_pos: Vector2) -> float:
	if _joystick != null and _joystick.get_active_area_global_rect().has_point(start_pos):
		return PINCH_JOYSTICK_MIN_TRAVEL
	return PINCH_MIN_FINGER_MOVE


func _commit_pinch(a: Vector2, b: Vector2) -> void:
	_pinch_active = true
	_pinch_prev_distance = a.distance_to(b)
	# Take the two fingers away from whatever they were driving.
	if _joystick:
		_joystick.cancel_gesture()
	_look_index = -1
	if _player:
		_player.begin_pinch_zoom()


func _reseat_pinch() -> void:
	var indices: Array = _touches.keys()
	if indices.size() < 2:
		return
	_pinch_a = indices[0]
	_pinch_b = indices[1]
	_pinch_prev_distance = (_touches[_pinch_a] as Vector2).distance_to(_touches[_pinch_b])


func _update_pinch() -> void:
	if not _touches.has(_pinch_a) or not _touches.has(_pinch_b):
		return
	var distance: float = (_touches[_pinch_a] as Vector2).distance_to(_touches[_pinch_b])
	var delta: float = distance - _pinch_prev_distance
	_pinch_prev_distance = distance
	if _player and not is_zero_approx(delta):
		_player.apply_pinch_zoom(delta)


func _end_pinch() -> void:
	_pinch_active = false
	_pinch_a = -1
	_pinch_b = -1
	# A surviving finger doesn't resume look until re-pressed (its press was
	# consumed while pinching) — cleared so the next fresh touch can look.
	_look_index = -1
	if _player:
		_player.end_pinch_zoom()


func _on_gui_input(event: InputEvent) -> void:
	# Chat open: a tap reaching this catcher means it wasn't consumed by the chat's
	# own controls, so close the chat instead of moving the camera.
	if _is_chat_visible() and event is InputEventScreenTouch and event.pressed:
		Global.close_chat.emit()
		accept_event()
		return
	# A tap reaching this catcher means no UI consumed it: reclaim ui_root focus so
	# movement re-enables if some control stole it without handing it back.
	if event is InputEventScreenTouch and event.pressed and not Global.explorer_has_focus():
		Global.explorer_grab_focus()
	if Global.scene_runner.raycast_use_cursor_position:
		_handle_cinematic(event)
		return
	if _player == null:
		return
	# A committed pinch consumes its fingers in _input, so they never arrive here;
	# single-finger look just tracks whichever free finger it first sees.
	if event is InputEventScreenTouch:
		if event.pressed:
			if _look_index == -1:
				_look_index = event.index
		elif event.index == _look_index:
			_look_index = -1
		accept_event()
	elif event is InputEventScreenDrag:
		if not _pinch_active and event.index == _look_index:
			_player.apply_look_delta(event.relative)
		accept_event()


func _handle_cinematic(event: InputEvent) -> void:
	if not (event is InputEventScreenTouch):
		return
	var explorer := Global.get_explorer()
	if explorer:
		explorer.set_cursor_position(event.position)
	if event.pressed:
		Input.action_press("ia_pointer")
	else:
		Input.action_release("ia_pointer")


# --- Gesture handoff from scene UI ---------------------------------------------
# DclUiControl owns the touch (Godot drag-capture) once a finger presses an
# interactive scene-UI element. When the finger swipes off the element it forwards
# the gesture here, routed by where the finger first touched down.


## Adopt a gesture that broke out of a scene-UI element. `index` is the touch
## index; `start_position` is the original press point (decides camera vs joystick
## and seeds the joystick base); `current_position`/`relative` describe the drag at
## the breakout moment.
func adopt_touch(
	index: int, start_position: Vector2, current_position: Vector2, relative: Vector2
) -> void:
	if Global.scene_runner.raycast_use_cursor_position:
		return
	if _player == null:
		return
	if _joystick and _joystick.get_active_area_global_rect().has_point(start_position):
		_adopted[index] = AdoptedMode.JOYSTICK
		_joystick.external_begin(start_position)
		_joystick.external_update(current_position)
	else:
		_adopted[index] = AdoptedMode.CAMERA
		_player.apply_look_delta(relative)


func update_adopted_touch(index: int, position: Vector2, relative: Vector2) -> void:
	match _adopted.get(index, AdoptedMode.NONE):
		AdoptedMode.CAMERA:
			if _player:
				_player.apply_look_delta(relative)
		AdoptedMode.JOYSTICK:
			if _joystick:
				_joystick.external_update(position)


func release_adopted_touch(index: int) -> void:
	_end_adopted(index)


func _end_adopted(index: int) -> void:
	if _adopted.get(index, AdoptedMode.NONE) == AdoptedMode.JOYSTICK and _joystick:
		_joystick.external_end()
	_adopted.erase(index)
