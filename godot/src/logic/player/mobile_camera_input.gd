class_name MobileCameraInput
extends Control

# Gesture adopted from scene UI (finger pressed a UI element then swiped off it).
# Kept separate from the gui_input state below so it never collides with normal
# touches handled by this catcher.
enum AdoptedMode { NONE, CAMERA, JOYSTICK }

const HORIZONTAL_SENS: float = 0.5
const VERTICAL_SENS: float = 0.5

# --- Pinch-to-zoom recognizer (issue #2636 follow-up) ------------------------
# Roblox model: pinch belongs to the CAMERA area only — the joystick's active rect
# is movement (walk + its own second-finger look), so a touch there is never a
# pinch candidate. That alone kills the walk-left + look-right false positive (the
# walk finger simply isn't eligible), so no reach/clampzone heuristic is needed.
# Both candidate fingers live in the camera area; the pinch and the joystick run
# independently (you can walk and zoom at once).
#
# With no competing gesture in the camera area, a pinch is simply the finger spread
# changing past this threshold — no "both fingers must move" / axial gate (those were
# anti-walk+look heuristics, now moot), so a thumb-anchored or angled pinch is caught.
const PINCH_COMMIT_SPREAD: float = 16.0

var _player: Player = null
var _chat_panel: Control = null
var _joystick: VirtualJoystick = null

# Single-finger camera look. Driven from gui_input (free touches only), so a
# finger over the joystick / a button / scene UI never looks. Independent from the
# global pinch tracking below.
var _look_index: int = -1
# Roblox-style: a SECOND finger INSIDE the joystick's active area drives the camera
# (the thumbstick keeps only its first finger). The joystick ignores extra touches
# and its STOP filter eats them from gui_input, so this look is driven from the
# global _input where the touch is still visible.
var _js_look_index: int = -1

# Every active touch (index → position), tracked in _input. Positions live here;
# _free_touches (below) decides which are pinch-eligible.
var _touches: Dictionary = {}
# Touches that reached _on_gui_input, i.e. NOT consumed by any UI (scene UI, HUD
# panels, chat, the joystick). Only these are pinch candidates, so a pinch over
# interactive UI never steals its drags. Added in gui_input, dropped on release.
var _free_touches: Dictionary = {}
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
		_free_touches.clear()
		_look_index = -1
		_js_look_index = -1
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

	# Releases are processed FIRST, unconditionally — a touch-up arriving during
	# cinematic/pointer mode (or before the player resolves) must never be dropped,
	# or a phantom finger lingers and a lone real finger confirms a spurious pinch.
	if event is InputEventScreenTouch and not event.pressed:
		var was_pinch_finger: bool = (
			_pinch_active and (event.index == _pinch_a or event.index == _pinch_b)
		)
		_touches.erase(event.index)
		_free_touches.erase(event.index)
		if event.index == _js_look_index:
			_js_look_index = -1
		_on_touch_count_changed()
		# Swallow a pinch finger's release so gui_input doesn't read it as a look end.
		if was_pinch_finger:
			get_viewport().set_input_as_handled()
		return

	if _player == null:
		return
	# No zoom in cinematic / pointer mode — gui_input drives the pointer there.
	if Global.scene_runner.raycast_use_cursor_position:
		return

	if event is InputEventScreenTouch:  # pressed
		_touches[event.index] = event.position
		_on_touch_count_changed()
	elif event is InputEventScreenDrag:
		_touches[event.index] = event.position
		if _pinch_active:
			if event.index == _pinch_a or event.index == _pinch_b:
				_update_pinch()
				get_viewport().set_input_as_handled()
			return
		# Re-seed on drag too: the free-touch set is filled by gui_input, which runs
		# AFTER _input, so a just-pressed free finger may not have been eligible on
		# its press event.
		if _pinch_a == -1 or _pinch_b == -1:
			_on_touch_count_changed()
		if _pinch_a != -1 and _pinch_b != -1:
			_try_recognize_pinch()
			if _pinch_active:
				get_viewport().set_input_as_handled()
				return
		_drive_joystick_area_look(event)


# (Re)establish the candidate pair when the touch count changes; end the pinch
# when it drops below two.
func _on_touch_count_changed() -> void:
	# Only FREE touches are pinch candidates (not the joystick, scene UI, HUD, chat).
	var cam: Array = _free_pinch_candidates()
	if cam.size() < 2:
		if _pinch_active:
			_end_pinch()
		else:
			_pinch_a = -1
			_pinch_b = -1
		return
	if not _pinch_active:
		_seed_pinch_candidate(cam)
	elif not _touches.has(_pinch_a) or not _touches.has(_pinch_b):
		# A committed pinch finger lifted but two camera-area ones remain: re-seat
		# onto the survivors so the gesture keeps going instead of stranding.
		_reseat_pinch(cam)


func _in_joystick_area(pos: Vector2) -> bool:
	return _joystick != null and _joystick.get_active_area_global_rect().has_point(pos)


# Free touch indices — those seen by gui_input, so owned by no UI (not the joystick,
# scene UI, HUD or chat). Only these can form a pinch.
func _free_pinch_candidates() -> Array:
	var out: Array = []
	for idx in _free_touches:
		if _touches.has(idx):
			out.append(idx)
	return out


func _seed_pinch_candidate(cam: Array) -> void:
	_pinch_a = cam[0]
	_pinch_b = cam[1]
	_pinch_start_a = _touches[_pinch_a]
	_pinch_start_b = _touches[_pinch_b]
	_pinch_prev_distance = _pinch_start_a.distance_to(_pinch_start_b)


func _try_recognize_pinch() -> void:
	if not _touches.has(_pinch_a) or not _touches.has(_pinch_b):
		return
	var a: Vector2 = _touches[_pinch_a]
	var b: Vector2 = _touches[_pinch_b]
	# The finger spread must change past the threshold from where the pair formed.
	var start_distance: float = _pinch_start_a.distance_to(_pinch_start_b)
	var distance: float = a.distance_to(b)
	if absf(distance - start_distance) >= PINCH_COMMIT_SPREAD:
		_commit_pinch(a, b)


## Drive the camera from a second finger inside the joystick's active area. Latches
## onto the first such finger (lazily, so a two-finger simultaneous touchdown still
## catches it once the joystick has claimed its primary) and looks until it lifts or
## a pinch takes over.
func _drive_joystick_area_look(event: InputEventScreenDrag) -> void:
	if _joystick == null or _player == null:
		return
	if _js_look_index == -1:
		if (
			_joystick.touch_index != -1
			and event.index != _joystick.touch_index
			and _joystick.get_active_area_global_rect().has_point(event.position)
		):
			_js_look_index = event.index
	if event.index == _js_look_index:
		_player.apply_look_delta(event.relative)


func _commit_pinch(a: Vector2, b: Vector2) -> void:
	_pinch_active = true
	_pinch_prev_distance = a.distance_to(b)
	# Both fingers are in the camera area (never the joystick's), so the walk keeps
	# running — only the free single-finger look yields to the zoom.
	_look_index = -1
	_js_look_index = -1
	if _player:
		_player.begin_pinch_zoom()


func _reseat_pinch(cam: Array) -> void:
	var indices: Array = cam
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
	# single-finger look just tracks whichever free finger it first sees. Reaching
	# gui_input at all marks the touch FREE (no UI consumed it) → pinch-eligible.
	if event is InputEventScreenTouch:
		if event.pressed:
			_free_touches[event.index] = true
			if _look_index == -1:
				_look_index = event.index
		else:
			_free_touches.erase(event.index)
			if event.index == _look_index:
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
