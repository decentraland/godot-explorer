class_name FtueTutorialRunner
extends Node

## Step state machine of the guided tutorial (issue #2767). Each step shows one instruction
## and ends only when the player does what it asks; see the table in _enter_step().
##
## Lives as a child of the FtueTutorialOverlay it drives, so freeing the overlay ends the run.

signal step_entered(step: Step)
signal completed
signal skipped(step: Step)

enum Step { MOVE, CAMERA, PINCH, CHAT, MENU, SOCIAL, BACKPACK, EQUIP, DONE }
# Which parts of the in-world HUD a step leaves on screen.
enum Hud { FULL, JOYSTICK_ONLY, ACTIONS_ONLY }

# i18n-keys: FTUE_TUTORIAL_MOVE, FTUE_TUTORIAL_CAMERA, FTUE_TUTORIAL_PINCH, FTUE_TUTORIAL_CHAT
# i18n-keys: FTUE_TUTORIAL_MENU, FTUE_TUTORIAL_SOCIAL, FTUE_TUTORIAL_BACKPACK, FTUE_TUTORIAL_EQUIP
const STEP_NAMES := {
	Step.MOVE: "move",
	Step.CAMERA: "camera",
	Step.PINCH: "pinch",
	Step.CHAT: "chat",
	Step.MENU: "menu",
	Step.SOCIAL: "social",
	Step.BACKPACK: "backpack",
	Step.EQUIP: "equip",
}
const JOYSTICK_RING_DIAMETER := 194.0
const BUTTON_RING_MARGIN := 22.0
const HOLE_MARGIN := 8.0
# A larger jump in one frame is a teleport, not walking.
const MAX_STEP_METERS := 2.0
# Lets the navbar finish its open animation before the next highlight is measured.
const NAVBAR_OPEN_SECONDS := 0.25
# Time for a menu, the profile, the chat or the navbar to finish closing or opening.
const SETTLE_SECONDS := 0.4
# Where the camera and pinch steps put their tooltip and gesture icon, as fractions of the screen.
const GESTURE_TOOLTIP_AT := Vector2(0.5, 0.24)
const GESTURE_AT := Vector2(0.66, 0.62)

var step := Step.MOVE
# Last step reported through step_entered; a step re-entered while it settles is not reported
# twice.
var _announced_step := -1

var _explorer: Explorer
var _overlay: FtueTutorialOverlay
var _player: Player
var _mount: Node3D
var _joystick: VirtualJoystick
# Untyped: these explorer nodes have no class_name to cast to.
var _navbar
var _friends_panel
var _move_meters := 1.0
var _camera_degrees := 30.0
var _social_seconds := 3.0

var _hud := Hud.FULL
var _suspended := false
var _last_position := Vector3.ZERO
var _moved := 0.0
var _last_yaw := 0.0
var _last_pitch := 0.0
var _rotated := 0.0
var _last_camera_mode := Global.CameraMode.THIRD_PERSON
var _seen_first_person := false
var _seen_third_person := false
# Second half of a step: the thing was opened (chat, social, backpack) or equipped.
var _engaged := false
# A re-entry of the current step is scheduled, to let something finish closing or opening.
var _settling := false
var _backpack: Backpack
# What the avatar wore when the equip step began.
var _outfit_before := PackedStringArray()
var _body_shape_before := ""


func start(
	explorer: Explorer,
	overlay: FtueTutorialOverlay,
	move_meters: float,
	camera_degrees: float,
	social_seconds: float
) -> void:
	_explorer = explorer
	_overlay = overlay
	_player = explorer.player as Player
	_mount = _player.mount_camera as Node3D
	_joystick = explorer.virtual_joystick as VirtualJoystick
	_navbar = explorer.navbar
	_friends_panel = explorer.friends_panel
	_move_meters = move_meters
	_camera_degrees = camera_degrees
	_social_seconds = social_seconds

	_overlay.skip_pressed.connect(skip)
	_overlay.resized.connect(_place_skip_button)
	Global.loading_started.connect(_on_loading_started)
	Global.loading_finished.connect(_on_loading_finished)
	Global.camera_mode_set.connect(_on_camera_mode_set)
	Global.open_backpack.connect(_on_backpack_requested)
	Global.on_menu_close.connect(_on_menu_closed)
	Global.friendship_request_sent.connect(_on_friendship_request_sent)
	var chat = explorer.chat_panel.chat
	chat.on_open_chat.connect(_on_chat_opened)
	chat.on_exit_chat.connect(_on_chat_closed)
	_navbar.navbar_opened.connect(_on_navbar_opened)
	_friends_panel.visibility_changed.connect(_on_friends_panel_visibility_changed)

	_place_skip_button.call_deferred()
	_enter_step(Step.MOVE)


func skip() -> void:
	var at := step
	_leave_world_ui()
	skipped.emit(at)


# Steps 1-3 hide most of the HUD, and explorer re-shows parts of it from several of its own
# handlers; re-asserting every frame is simpler than hooking each one.
func _process(_delta: float) -> void:
	if _suspended:
		return
	_apply_hud()
	match step:
		Step.MOVE:
			# Released, the joystick stops being drawn; the step points at it, so it must show.
			_joystick.show_resting()
			var position := _player.global_position
			var travelled := Vector2(position.x - _last_position.x, position.z - _last_position.z)
			_last_position = position
			if travelled.length() < MAX_STEP_METERS:
				_moved += travelled.length()
			if _moved >= _move_meters:
				_enter_step(Step.CAMERA)
		Step.CAMERA:
			var yaw := _player.rotation.y
			var pitch := _mount.rotation.x
			_rotated += absf(angle_difference(_last_yaw, yaw)) + absf(pitch - _last_pitch)
			_last_yaw = yaw
			_last_pitch = pitch
			if rad_to_deg(_rotated) >= _camera_degrees:
				_enter_step(Step.PINCH)
		Step.CHAT, Step.MENU:
			_keep_settled(false)
		Step.SOCIAL:
			_keep_settled(true)
		Step.BACKPACK:
			if _engaged:
				_poll_backpack_ready()
			else:
				_keep_settled(true)
		Step.EQUIP:
			if not _engaged and is_instance_valid(_backpack):
				var hole := _backpack.scroll_container_items.get_global_rect()
				_overlay.show_dim(hole, 12.0)
				_overlay.block_input_except(hole)
				if _has_equipped_something():
					_engage()


func _enter_step(next: Step) -> void:
	step = next
	_engaged = false
	_settling = false
	_overlay.ring.stop()
	_overlay.hide_dim()
	_overlay.hide_tooltip()
	_overlay.hide_gesture()
	_overlay.free_input()
	_place_skip_button()
	if next != Step.DONE and next != _announced_step:
		_announced_step = next
		step_entered.emit(next)

	var view := _overlay.size
	match next:
		Step.MOVE:
			if Global.touch_controls_hide_joystick:
				_enter_step.call_deferred(Step.CAMERA)
				return
			_hud = Hud.JOYSTICK_ONLY
			_last_position = _player.global_position
			_moved = 0.0
			var center := _joystick.get_base_global_center()
			var radius := JOYSTICK_RING_DIAMETER * 0.5
			# Only the joystick's area takes touches, so a drag elsewhere cannot turn the
			# camera while this step is about moving.
			_overlay.block_input_except(_joystick.get_active_area_global_rect())
			_overlay.ring.play(center, JOYSTICK_RING_DIAMETER)
			_overlay.show_tooltip(
				"FTUE_TUTORIAL_MOVE",
				Rect2(center - Vector2(radius, radius), Vector2(radius, radius) * 2.0),
				FtueTutorialOverlay.Place.ABOVE
			)
		Step.CAMERA:
			_hud = Hud.ACTIONS_ONLY
			_last_yaw = _player.rotation.y
			_last_pitch = _mount.rotation.x
			_rotated = 0.0
			_overlay.show_tooltip(
				"FTUE_TUTORIAL_CAMERA",
				Rect2(view * GESTURE_TOOLTIP_AT, Vector2.ZERO),
				FtueTutorialOverlay.Place.CENTER
			)
			_overlay.show_gesture(FtueTutorialOverlay.Gesture.DRAG, view * GESTURE_AT)
		Step.PINCH:
			# Pinch needs a touchscreen, and does nothing while a scene locks the camera mode.
			if not DisplayServer.is_touchscreen_available() or Global.camera_mode_blocked:
				_enter_step.call_deferred(Step.CHAT)
				return
			_hud = Hud.ACTIONS_ONLY
			_last_camera_mode = Global.current_camera_mode
			_seen_first_person = false
			_seen_third_person = false
			_overlay.show_tooltip(
				"FTUE_TUTORIAL_PINCH",
				Rect2(view * GESTURE_TOOLTIP_AT, Vector2.ZERO),
				FtueTutorialOverlay.Place.CENTER
			)
			_overlay.show_gesture(FtueTutorialOverlay.Gesture.PINCH, view * GESTURE_AT)
		Step.CHAT:
			if _settle(false):
				_highlight_button(
					_chat_button(), "FTUE_TUTORIAL_CHAT", FtueTutorialOverlay.Place.BELOW
				)
		Step.MENU:
			if _settle(false):
				_highlight_button(
					_menu_button(), "FTUE_TUTORIAL_MENU", FtueTutorialOverlay.Place.BELOW
				)
		Step.SOCIAL:
			_highlight_in_navbar("%StaticButton_Friends", "FTUE_TUTORIAL_SOCIAL")
		Step.BACKPACK:
			_highlight_in_navbar("%StaticButton_Backpack", "FTUE_TUTORIAL_BACKPACK")
		Step.EQUIP:
			_overlay.show_tooltip(
				"FTUE_TUTORIAL_EQUIP",
				_backpack.scroll_container_items.get_global_rect(),
				FtueTutorialOverlay.Place.LEFT
			)
			var avatar = Global.player_identity.get_mutable_avatar()
			_outfit_before = avatar.get_wearables().duplicate()
			_body_shape_before = avatar.get_body_shape()
		Step.DONE:
			_restore_hud()
			completed.emit()


func _highlight_button(
	button: Control, tooltip_key: String, place: FtueTutorialOverlay.Place
) -> void:
	var rect := button.get_global_rect()
	_overlay.show_dim(rect.grow(HOLE_MARGIN), rect.size.x)
	_overlay.block_input_except(rect)
	_overlay.ring.play(rect.get_center(), maxf(rect.size.x, rect.size.y) + BUTTON_RING_MARGIN)
	_overlay.show_tooltip(tooltip_key, rect, place)


# Steps 6 and 7 point at an entry of the open navbar: the whole menu bar stays lit, only the
# entry takes touches.
func _highlight_in_navbar(button_path: String, tooltip_key: String) -> void:
	if not _settle(true):
		return
	var button: Control = _navbar.get_node(button_path)
	var rect := button.get_global_rect()
	var bar: Control = _navbar.get_node("Control_Menu/PanelContainer")
	_overlay.show_dim(bar.get_global_rect(), 24.0)
	_overlay.block_input_except(rect)
	_overlay.ring.play(rect.get_center(), maxf(rect.size.x, rect.size.y) + BUTTON_RING_MARGIN)
	_overlay.show_tooltip(tooltip_key, rect, FtueTutorialOverlay.Place.RIGHT)


# Brings the HUD to the state a step starts from. While a step lets the player use what it
# opened (chat, social) they can leave anything open: a menu, the profile, the navbar. One
# thing is put right per pass and the step is re-entered once it has had time to close; nothing
# is touchable meanwhile. Returns true when there is nothing left to fix.
func _settle(navbar_open: bool) -> bool:
	_restore_hud()
	if _explorer.control_menu.visible:
		Global.close_menu.emit()
	elif _explorer.profile_container.visible:
		_explorer.profile_container.call("close")
	elif _explorer.chat_panel.is_chat_visible():
		Global.close_chat.emit()
	elif _navbar.is_open() != navbar_open:
		# Routed through the toggle so explorer runs its usual navbar handling.
		_menu_button().button_pressed = navbar_open
	else:
		return true
	_settling = true
	_overlay.block_input_except(Rect2())
	_enter_step_after(step, SETTLE_SECONDS)
	return false


# A screen can still open after a step settled (the profile loads before it shows), which
# would leave the highlight pointing at something covered. Checked every frame while the
# step waits for its tap.
func _keep_settled(navbar_open: bool) -> void:
	if _engaged or _settling:
		return
	if (
		_explorer.control_menu.visible
		or _explorer.profile_container.visible
		or _explorer.chat_panel.is_chat_visible()
		or _navbar.is_open() != navbar_open
	):
		_enter_step(step)


func _enter_step_after(next: Step, seconds: float) -> void:
	var from := step
	get_tree().create_timer(seconds).timeout.connect(
		func() -> void:
			if step == from and not _suspended:
				_enter_step(next)
	)


func _apply_hud() -> void:
	if _hud == Hud.FULL:
		return
	_explorer.hud_content.visible = false
	_explorer.joypad.visible = _hud == Hud.ACTIONS_ONLY
	_explorer.virtual_joystick.visible = _hud == Hud.JOYSTICK_ONLY


func _restore_hud() -> void:
	if _hud == Hud.FULL:
		return
	_hud = Hud.FULL
	_explorer.set_visible_ui(true, true)
	_explorer.mobile_ui.show()
	_explorer.joypad.show()
	_explorer.virtual_joystick.show()


# Back to the world with nothing of the tutorial's making left open.
func _leave_world_ui() -> void:
	_restore_hud()
	if _explorer.chat_panel.is_chat_visible():
		Global.close_chat.emit()
	if _explorer.control_menu.visible:
		Global.close_menu.emit()
	if _navbar.is_open():
		_navbar.collapse()


# The backpack's category tabs sit in the top-right corner.
func _place_skip_button() -> void:
	_overlay.place_skip_button(_menu_button().get_global_rect(), step == Step.EQUIP)


func _menu_button() -> Button:
	return _navbar.get_node("%Button")


func _chat_button() -> Control:
	return _explorer.chat_panel.get_node("%Chatbar").get_node("%Button_Chat")


func _on_loading_started() -> void:
	_suspended = true
	_overlay.hide()


func _on_loading_finished() -> void:
	if not _suspended:
		return
	_suspended = false
	_overlay.show()
	_enter_step(Step.BACKPACK if step == Step.EQUIP else step)


func _on_camera_mode_set(camera_mode: Global.CameraMode) -> void:
	if step != Step.PINCH or camera_mode == _last_camera_mode:
		return
	_last_camera_mode = camera_mode
	_seen_first_person = _seen_first_person or camera_mode == Global.CameraMode.FIRST_PERSON
	_seen_third_person = _seen_third_person or camera_mode == Global.CameraMode.THIRD_PERSON
	if _seen_first_person and _seen_third_person:
		_enter_step(Step.CHAT)


func _on_chat_opened() -> void:
	if step != Step.CHAT or _engaged:
		return
	_engage()


func _on_chat_closed() -> void:
	if step == Step.CHAT and _engaged:
		_enter_step(Step.MENU)


func _on_navbar_opened() -> void:
	if step == Step.MENU and not _settling:
		_engaged = true
		_enter_step_after(Step.SOCIAL, NAVBAR_OPEN_SECONDS)


func _on_friends_panel_visibility_changed() -> void:
	if step != Step.SOCIAL or _engaged or not _friends_panel.visible:
		return
	_engage()
	# New players have no friends yet; Nearby is where they can add some.
	_select_nearby_tab.call_deferred()
	_enter_step_after(Step.BACKPACK, _social_seconds)


# Deferred from the visibility change: the panel picks its own default tab right after showing.
func _select_nearby_tab() -> void:
	_friends_panel.button_nearby.button_pressed = true


func _on_friendship_request_sent(_address: String) -> void:
	if step == Step.SOCIAL and _engaged:
		_enter_step(Step.BACKPACK)


func _on_backpack_requested(_on_emotes: bool) -> void:
	if step != Step.BACKPACK or _engaged:
		return
	_engage()
	# Nothing may be touched until the backpack exists and has listed its items.
	_overlay.block_input_except(Rect2())


func _poll_backpack_ready() -> void:
	var responsive = _explorer.control_menu.control_backpack.instance
	if not is_instance_valid(responsive):
		return
	var backpack = responsive.backpack_landscape.instance
	if not is_instance_valid(backpack) or not backpack.is_node_ready():
		return
	_backpack = backpack as Backpack
	# The "no items" placeholder is also what the backpack shows while it is still fetching, so
	# it cannot tell an empty backpack from a loading one; an empty one is left to SKIP.
	if _backpack.grid_container_wearables_list.get_child_count() > 0:
		_enter_step(Step.EQUIP)


# Compared against the avatar, not the items' `equip` signal: the grid emits that one itself
# for everything already worn while it fills.
func _has_equipped_something() -> bool:
	var avatar = Global.player_identity.get_mutable_avatar()
	if avatar.get_body_shape() != _body_shape_before:
		return true
	for urn in avatar.get_wearables():
		if not _outfit_before.has(urn):
			return true
	return false


func _on_menu_closed() -> void:
	if step == Step.EQUIP and _engaged:
		_enter_step(Step.DONE)


# The player did what the step pointed at: drop the hints and let them use it freely.
func _engage() -> void:
	_engaged = true
	_overlay.ring.stop()
	_overlay.hide_dim()
	_overlay.hide_tooltip()
	_overlay.free_input()
