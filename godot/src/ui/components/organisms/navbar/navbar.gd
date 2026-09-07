extends Control

signal close_only_panels
signal navbar_opened
signal navbar_closed

enum BUTTON { FRIENDS, NOTIFICATIONS, BACKPACK, SETTINGS }

# Open/close timeline tuning. The navbar dropdown fades its alpha in; the side-panel surface
# grows from the navbar edge to its full width (scale.x 0 -> 1), leading with the fade.
const FADE_TIME: float = 0.12
const GROW_TIME: float = 0.16
const GROW_DELAY: float = 0.04
const CLOSE_TIME: float = 0.1

var _manually_hidden: bool = false
# Navbar buttons ordered to match Control_Selection1..6 (top to bottom); filled in _ready.
var _selection_buttons: Array[BaseButton] = []
# The side-panel surface (VBoxContainer_LeftPanels) injected by explorer via set_reveal_surface.
# The navbar owns its reveal/collapse so the fade and the grow stay on one timeline. It is not a
# child of the navbar, so we only ever touch its scale/pivot — never its layout or visibility.
var _reveal_surface: Control = null
var _open_tween: Tween = null

@onready var v_box_container_buttons: VBoxContainer = %VBoxContainer_Buttons
@onready var static_button_friends: TextureButton = %StaticButton_Friends
@onready var static_button_notifications: TextureButton = %StaticButton_Notifications
@onready var button: Button = %Button
@onready var portrait_button_profile: TextureButton = %Portrait_Button_Profile
@onready var static_button_backpack: TextureButton = %StaticButton_Backpack
@onready var static_button_settings: TextureButton = %StaticButton_Settings
@onready var static_button_discover: TextureButton = %StaticButton_Discover
@onready var panel_profile: ProfileIconButton = %Panel_Profile

# The dropdown that fades in (%Control_Menu) and the collapsed profile toggle it swaps with.
@onready var _dropdown: Control = $Control_Menu
@onready var _collapsed: Control = $Control

# One shared selection effect (%Control_Effect) that reparents into the slot of the pressed
# button. `_selection_buttons` and `_selection_slots` are index-aligned, top-to-bottom.
@onready var _selection_effect: Control = %Control_Effect
@onready var _selection_slots: Array[Control] = [
	%Control_Selection1,
	%Control_Selection2,
	%Control_Selection3,
	%Control_Selection4,
	%Control_Selection5,
	%Control_Selection6,
]


func _ready() -> void:
	var btn_group = ButtonGroup.new()
	btn_group.allow_unpress = false
	# Ordered to match Control_Selection1..6 in VBoxContainer_Selection (top to bottom).
	_selection_buttons = [
		static_button_discover,
		static_button_friends,
		static_button_notifications,
		static_button_backpack,
		static_button_settings,
		portrait_button_profile,
	]
	for selection_button in _selection_buttons:
		selection_button.button_group = btn_group
	# The ButtonGroup with allow_unpress = false ensures one is always pressed; move the shared
	# selection effect into the pressed button's slot, now and on every change.
	btn_group.pressed.connect(_on_selection_button_pressed)
	var pressed_button: BaseButton = btn_group.get_pressed_button()
	if pressed_button != null:
		_on_selection_button_pressed(pressed_button)

	Global.close_navbar.connect(_on_navbar_close)
	Global.open_navbar_silently.connect(_on_navbar_open_silently_on_backpack)

	# Initial collapsed visuals (previously set by the AnimationPlayer's autoplay "close").
	_dropdown.visible = false
	_collapsed.visible = true

	# Sync the profile glow to the navbar's initial (collapsed) state so it doesn't
	# show before the first open/close toggle.
	panel_profile.set_glow(button.button_pressed)

	get_window().size_changed.connect(self._on_size_changed)
	_on_size_changed()


func _on_size_changed():
	if _manually_hidden:
		return
	# If navbar was manually hidden, don't change its visibility

	var explorer = Global.get_explorer()
	if explorer != null:
		# Check if discover or chat are open - if so, keep hidden
		if (
			explorer.control_menu != null
			and explorer.control_menu.visible
			and explorer.control_menu.control_discover.instance != null
			and explorer.control_menu.control_discover.instance.visible
		):
			# If discover is open, keep hidden
			hide()
			return
	var window_size: Vector2i = DisplayServer.window_get_size()
	visible = window_size.x > window_size.y


func _on_navbar_close() -> void:
	collapse()


## Reparents the shared selection effect into the slot matching the pressed navbar button, so the
## highlight follows the selection. Full-rect anchors let it fill whichever slot it lands in.
func _on_selection_button_pressed(pressed_button: BaseButton) -> void:
	var index: int = _selection_buttons.find(pressed_button)
	# Guard the slot lookup: _selection_buttons and _selection_slots are index-aligned, but a
	# future 7th button added to the group without a matching slot would index out of bounds.
	if index == -1 or index >= _selection_slots.size():
		return
	var target: Control = _selection_slots[index]
	if _selection_effect.get_parent() != target:
		_selection_effect.reparent(target, false)


## Injects the side-panel surface the navbar reveals on open. Called once by explorer. The surface
## grows from its left edge (adjacent to the navbar) via scale.x, so we pin its pivot there and
## start it collapsed. Scale is a GPU transform: no per-frame relayout while it animates.
func set_reveal_surface(surface: Control) -> void:
	_reveal_surface = surface
	if _reveal_surface != null:
		_reveal_surface.pivot_offset = Vector2.ZERO
		_reveal_surface.scale = Vector2(0.0, 1.0)


func _kill_open_tween() -> void:
	if _open_tween != null and _open_tween.is_valid():
		_open_tween.kill()
	_open_tween = null


## Fade the dropdown in, then grow the side-panel surface from the navbar edge. Panel content is
## already shown by explorer (surface starts at scale.x 0, so it's revealed by the grow).
func _animate_open() -> void:
	_kill_open_tween()
	_collapsed.visible = false
	_dropdown.visible = true
	_dropdown.modulate.a = 0.0
	_open_tween = create_tween()
	_open_tween.set_parallel(true)
	_open_tween.tween_property(_dropdown, "modulate:a", 1.0, FADE_TIME)
	if _reveal_surface != null:
		_reveal_surface.scale = Vector2(0.0, 1.0)
		(
			_open_tween
			. tween_property(_reveal_surface, "scale:x", 1.0, GROW_TIME)
			. set_delay(GROW_DELAY)
			. set_trans(Tween.TRANS_CUBIC)
			. set_ease(Tween.EASE_OUT)
		)


## Collapse the surface and fade the dropdown out, then restore the collapsed profile toggle.
func _animate_close() -> void:
	_kill_open_tween()
	_open_tween = create_tween()
	_open_tween.set_parallel(true)
	_open_tween.tween_property(_dropdown, "modulate:a", 0.0, CLOSE_TIME)
	if _reveal_surface != null:
		(
			_open_tween
			. tween_property(_reveal_surface, "scale:x", 0.0, CLOSE_TIME)
			. set_trans(Tween.TRANS_CUBIC)
			. set_ease(Tween.EASE_IN)
		)
	_open_tween.chain().tween_callback(_finish_close)


func _finish_close() -> void:
	_dropdown.visible = false
	_collapsed.visible = true


func _on_button_toggled(toggled_on: bool) -> void:
	Global.send_haptic_feedback()
	panel_profile.set_glow(toggled_on)
	if toggled_on:
		# Grab the frame BEFORE the open animation starts: this is the last
		# moment the world is unobstructed, and it is what the bug report form
		# pre-fills its screenshot slot with (issue #2652).
		BugReportCapture.capture(get_viewport())
		set_button_pressed(BUTTON.FRIENDS)
		navbar_opened.emit()
		_animate_open()
	else:
		navbar_closed.emit()
		_animate_close()


## Set a button as pressed
func set_button_pressed(button_to_press: BUTTON) -> void:
	match button_to_press:
		BUTTON.FRIENDS:
			static_button_friends.button_pressed = true
		BUTTON.NOTIFICATIONS:
			static_button_notifications.button_pressed = true
		BUTTON.BACKPACK:
			static_button_backpack.button_pressed = true
		BUTTON.SETTINGS:
			static_button_settings.button_pressed = true


func capture_mouse():
	if DisplayServer.has_feature(DisplayServer.FEATURE_MOUSE):
		Input.set_mouse_mode(Input.MOUSE_MODE_CAPTURED)


func collapse():
	button.set_pressed_no_signal(false)
	panel_profile.set_glow(false)
	navbar_closed.emit()
	_animate_close()


## True while the navbar dropdown is expanded. Lets callers collapse it only when
## needed, avoiding a redundant navbar_closed emit (and its panel teardown).
func is_open() -> bool:
	return button.button_pressed


func _on_navbar_open_silently_on_backpack() -> void:
	open_navbar_silently()
	set_button_pressed(BUTTON.BACKPACK)


func open_navbar_silently() -> void:
	if not button.button_pressed:
		button.set_pressed_no_signal(true)
		panel_profile.set_glow(true)
		_animate_open()


func set_manually_hidden(is_hidden: bool) -> void:
	_manually_hidden = is_hidden
	if is_hidden:
		hide()
	else:
		var explorer = Global.get_explorer()
		if explorer != null:
			# Check if discover or chat are open before restoring visibility
			if (
				explorer.control_menu != null
				and explorer.control_menu.visible
				and explorer.control_menu.control_discover.instance != null
				and explorer.control_menu.control_discover.instance.visible
			):
				# If discover is open, keep hidden
				return
		# Restore visibility based on window size
		var window_size: Vector2i = DisplayServer.window_get_size()
		visible = window_size.x > window_size.y
