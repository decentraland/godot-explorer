extends Control

const GLIDER_ICON_MAX_WIDTH = 85
const GLIDER_ICON = preload("uid://dnosnq2stqu11")  # "res://assets/themes/dark_dcl_theme/icons/Glider.svg"

const DOUBLE_JUMP_ICON_MAX_WIDTH = 52
const DOUBLE_JUMP_ICON = preload("uid://euvimxirt85b")  # "res://assets/themes/dark_dcl_theme/icons/DoubleJump.svg"

const SINGLE_JUMP_ICON_MAX_WIDTH = 85
const SINGLE_JUMP_ICON = preload("uid://ck3atqpytstpo")  # "res://assets/themes/dark_dcl_theme/icons/Jump.svg"

const TOUCHABLE_NORMAL_STYLEBOX = preload("uid://b66geet5bo5yf")  # touchable_normal.tres (black bg)
const TOUCHABLE_PRESSED_STYLEBOX = preload("uid://cvducxvis7n6e")  # touchable_pressed.tres (white bg)

const TOUCHABLE_ICON_LIGHT := Color(0.9882353, 0.9882353, 0.9882353, 1)
const TOUCHABLE_ICON_DARK := Color(0, 0, 0, 0.7019608)

const INVERTED_NORMAL_STYLES: Array[StringName] = [
	&"normal", &"normal_mirrored", &"hover", &"hover_mirrored"
]
const INVERTED_PRESSED_STYLES: Array[StringName] = [
	&"pressed", &"pressed_mirrored", &"hover_pressed", &"hover_pressed_mirrored"
]
const INVERTED_NORMAL_ICON_COLORS: Array[StringName] = [&"icon_normal_color", &"icon_hover_color"]
const INVERTED_PRESSED_ICON_COLORS: Array[StringName] = [
	&"icon_pressed_color", &"icon_hover_pressed_color"
]

const ICON_SINGLE_JUMP := 0
const ICON_DOUBLE_JUMP := 1
const ICON_GLIDER := 2

const DEFAULT_MAIN_ACTION := "ia_jump"

var combo_opened: bool = false

var _current_icon: int = ICON_DOUBLE_JUMP
var _showing_inverted_colors: bool = false

# PBTouchScreenControls (Global.touch_controls_*): denylist of on-screen buttons + a
# swappable central "main action". `_tc_action_buttons` maps an InputAction (godot action
# name) to its button; `_tc_default_icons` / `_tc_default_text` remember each button's
# default glyph (some buttons render via an icon, e.g. jump/pointer, others via text, e.g.
# E/F/1..4). Populated in _ready.
var _tc_action_buttons: Dictionary = {}
var _tc_default_icons: Dictionary = {}
var _tc_default_text: Dictionary = {}
var _tc_active_applied: bool = false
var _last_tc_hash: int = 0
var _main_action: String = DEFAULT_MAIN_ACTION
var _jump_icon_overridden: bool = false

@onready var animation_player: AnimationPlayer = %AnimationPlayer
@onready var button_combo: Button = %Button_Combo
@onready var button_press: Button = $Button_Press

@onready var _combo_action_buttons: Array[Button] = [
	%Button_Combo1,
	%Button_Combo2,
	%Button_Combo3,
	%Button_Combo4,
]


func _ready() -> void:
	for btn in _combo_action_buttons:
		btn.touch_action_changed.connect(_on_combo_action_changed)
	_set_attenuated_sound_for_buttons(self)
	_apply_jump_icon(ICON_DOUBLE_JUMP)

	_tc_action_buttons = {
		"ia_jump": button_press,
		"ia_pointer": $Button_Press/Control/HBoxContainer/Button_Jump,
		"ia_primary": $Button_Press/Control/HBoxContainer2/Button_Primary,
		"ia_secondary": $Button_Press/Control/HBoxContainer4/Button_Secondary,
		"ia_action_3": %Button_Combo1,
		"ia_action_4": %Button_Combo2,
		"ia_action_5": %Button_Combo3,
		"ia_action_6": %Button_Combo4,
	}
	for action in _tc_action_buttons:
		var btn := _tc_action_buttons[action] as Button
		_tc_default_icons[action] = btn.icon
		_tc_default_text[action] = btn.text


func _process(_dt: float) -> void:
	_update_jump_icon()
	_apply_touch_controls()


func _update_jump_icon() -> void:
	# The central button's icon is dynamic only while it is the jump button and not overridden.
	if _main_action != "ia_jump" or _jump_icon_overridden:
		return
	var player := Global.scene_runner.player_body_node as Player
	if player == null:
		return

	var glide_disabled_in_scene := Global.is_glide_disabled()
	var double_jump_disabled_in_scene := Global.is_double_jump_disabled()

	# Base icon picked from scene-level capabilities.
	var base_icon := ICON_SINGLE_JUMP if double_jump_disabled_in_scene else ICON_DOUBLE_JUMP

	var want_icon := _current_icon
	match player.get_jump_action():
		Player.JUMP_ACTION_JUMP:
			want_icon = base_icon
		Player.JUMP_ACTION_GLIDE_TOGGLE:
			# Defense in depth: never flip to glider if the current scene
			# disables it, even if the player's glide_state is still
			# OPENING/GLIDING for the one tick before force-close lands.
			want_icon = base_icon if glide_disabled_in_scene else ICON_GLIDER
		_:  # JUMP_ACTION_NONE
			# Keep the glider "sticky" through GLIDE_CLOSING to avoid a flicker
			# between GLIDING and the post-close cooldown — unless the scene
			# has just disabled glide, in which case drop it immediately.
			if _current_icon == ICON_GLIDER and glide_disabled_in_scene:
				want_icon = base_icon
			elif _current_icon != ICON_GLIDER:
				# Re-sync the base icon when scene-level flags change under
				# us (e.g. player walks into a parcel that disables double jump).
				want_icon = base_icon

	if want_icon != _current_icon:
		_apply_jump_icon(want_icon)

	# Inverted ("toggled") button styling only while the glider is actually
	# providing lift in an allowed scene. Suppress on scene-level disable so
	# the force-close transition doesn't flash the pressed state.
	var glide_active := (
		player.glide_state != Player.GLIDE_CLOSED
		and not (player.is_on_floor() or player.position.y <= 0.0)
		and not glide_disabled_in_scene
	)
	if glide_active != _showing_inverted_colors:
		_showing_inverted_colors = glide_active
		_apply_inverted_colors(glide_active)


func _apply_jump_icon(icon_id: int) -> void:
	_current_icon = icon_id
	match icon_id:
		ICON_GLIDER:
			button_press.icon = GLIDER_ICON
			button_press.add_theme_constant_override("icon_max_width", GLIDER_ICON_MAX_WIDTH)
		ICON_SINGLE_JUMP:
			button_press.icon = SINGLE_JUMP_ICON
			button_press.add_theme_constant_override("icon_max_width", SINGLE_JUMP_ICON_MAX_WIDTH)
		_:
			button_press.icon = DOUBLE_JUMP_ICON
			button_press.add_theme_constant_override("icon_max_width", DOUBLE_JUMP_ICON_MAX_WIDTH)


func _apply_inverted_colors(inverted: bool) -> void:
	if inverted:
		for style in INVERTED_NORMAL_STYLES:
			button_press.add_theme_stylebox_override(style, TOUCHABLE_PRESSED_STYLEBOX)
		for style in INVERTED_PRESSED_STYLES:
			button_press.add_theme_stylebox_override(style, TOUCHABLE_NORMAL_STYLEBOX)
		for color_name in INVERTED_NORMAL_ICON_COLORS:
			button_press.add_theme_color_override(color_name, TOUCHABLE_ICON_DARK)
		for color_name in INVERTED_PRESSED_ICON_COLORS:
			button_press.add_theme_color_override(color_name, TOUCHABLE_ICON_LIGHT)
	else:
		for style in INVERTED_NORMAL_STYLES:
			button_press.remove_theme_stylebox_override(style)
		for style in INVERTED_PRESSED_STYLES:
			button_press.remove_theme_stylebox_override(style)
		for color_name in INVERTED_NORMAL_ICON_COLORS:
			button_press.remove_theme_color_override(color_name)
		for color_name in INVERTED_PRESSED_ICON_COLORS:
			button_press.remove_theme_color_override(color_name)


func _set_attenuated_sound_for_buttons(node: Node) -> void:
	if node is Button:
		node.set_meta("attenuated_sound", true)

	for child in node.get_children():
		_set_attenuated_sound_for_buttons(child)


func _on_button_combo_toggled(toggled_on: bool) -> void:
	combo_opened = toggled_on
	if toggled_on:
		animation_player.play("open_combo")
	else:
		animation_player.play_backwards("open_combo")


func _on_combo_action_changed(pressed: bool) -> void:
	if not pressed and combo_opened:
		button_combo.toggled.emit(false)
		button_combo.set_pressed_no_signal(false)


## Applies PBTouchScreenControls (Global.touch_controls_*). No component (inactive) → the
## gamepad keeps its default layout. Active → every button is shown EXCEPT those flagged
## `hide` (denylist), `icon` overrides are applied, and `main_action` swaps the central
## button. The joypad governs its own visibility (shown on desktop too), so this runs
## regardless of platform.
func _apply_touch_controls() -> void:
	var active: bool = Global.touch_controls_active
	var inputs: Array = Global.touch_controls_inputs
	var main_action := String(Global.touch_controls_main_action)
	var state_hash := hash([active, inputs, main_action])
	if state_hash == _last_tc_hash:
		return
	_last_tc_hash = state_hash

	if not active:
		if _tc_active_applied:
			_tc_active_applied = false
			_restore_touch_controls()
		return

	_tc_active_applied = true

	var hidden := {}
	var icons := {}  # action -> { "hash": String, "url": String }
	for entry in inputs:
		var action := String(entry.get("action", ""))
		if bool(entry.get("hide", false)):
			hidden[action] = true
		var icon_hash := String(entry.get("icon_hash", ""))
		if not icon_hash.is_empty():
			icons[action] = {"hash": icon_hash, "url": String(entry.get("icon_url", ""))}

	var jump_icon: Dictionary = icons.get("ia_jump", {})
	_apply_main_action(
		main_action, String(jump_icon.get("hash", "")), String(jump_icon.get("url", ""))
	)

	for action in _tc_action_buttons:
		var btn := _tc_action_buttons[action] as Button
		var hide_it: bool = hidden.has(action)
		# The action promoted to the central button is suppressed in its normal slot.
		if _main_action != "ia_jump" and action == _main_action:
			hide_it = true
		# Combo (action_3..6) buttons stay .visible unless denylisted; the tray open/close
		# animation controls whether they're actually revealed (like the default gamepad).
		btn.visible = not hide_it
		# The central (jump) button's icon is handled by _apply_main_action.
		if action != "ia_jump":
			var icon: Dictionary = icons.get(action, {})
			_apply_icon_override(
				action, btn, String(icon.get("hash", "")), String(icon.get("url", ""))
			)

	# The combo tray toggle is only useful if at least one combo action is still visible.
	var any_combo := false
	for a in ["ia_action_3", "ia_action_4", "ia_action_5", "ia_action_6"]:
		if not hidden.has(a):
			any_combo = true
			break
	button_combo.visible = any_combo


func _restore_touch_controls() -> void:
	_apply_main_action("", "", "")
	for action in _tc_action_buttons:
		var btn := _tc_action_buttons[action] as Button
		if action != "ia_jump":
			_restore_button_default_style(btn, action)
		# Buttons stay .visible; the combo tray animation governs combo-button display.
		btn.visible = true
	button_combo.visible = true


## Sets the large central button to `main_action` (empty = default jump). `jump_icon_hash`/
## `jump_icon_url` are an optional custom icon applied only when the central button is jump.
func _apply_main_action(main_action: String, jump_icon_hash: String, jump_icon_url: String) -> void:
	var target := main_action if not main_action.is_empty() else DEFAULT_MAIN_ACTION
	_main_action = target
	button_press.trigger_action = target

	if target == "ia_jump":
		if jump_icon_hash.is_empty():
			# Hide any custom overlay and hand the icon back to the dynamic jump logic.
			_jump_icon_overridden = false
			button_press.set_meta("tc_icon_hash", "")
			if button_press.has_method("clear_custom_icon"):
				button_press.clear_custom_icon()
			button_press.text = _tc_default_text.get("ia_jump", "")
			_apply_jump_icon(_current_icon)
		else:
			# Keep the current jump icon as a placeholder while the custom one loads.
			_jump_icon_overridden = true
			_async_set_button_icon(button_press, jump_icon_hash, jump_icon_url)
	else:
		# Borrow the promoted action's default glyph (icon or text), from a clean state.
		_jump_icon_overridden = false
		_restore_button_default_style(button_press, target)


func _apply_icon_override(action: String, btn: Button, icon_hash: String, icon_url: String) -> void:
	if icon_hash.is_empty():
		_restore_button_default_style(btn, action)
		return
	# Clean default state as a placeholder while the custom icon loads.
	_restore_button_default_style(btn, action)
	_async_set_button_icon(btn, icon_hash, icon_url)


## Async-loads a scene content texture (by content hash + URL) into a button's icon, reusing
## the shared content provider. A per-button meta guards against stale applies (the config
## may change while awaiting). Falls back to the current glyph on cache-miss/failure.
func _async_set_button_icon(btn: Button, icon_hash: String, icon_url: String) -> void:
	btn.set_meta("tc_icon_hash", icon_hash)

	var cached: Texture2D = Global.content_provider.get_texture_from_hash(icon_hash)
	if cached != null:
		_apply_custom_icon(btn, cached)
		return

	var promise: Promise = Global.content_provider.fetch_texture_by_url(icon_hash, icon_url)
	var res = await PromiseUtils.async_awaiter(promise)
	# Bail if the button's desired icon changed while we were awaiting.
	if String(btn.get_meta("tc_icon_hash", "")) != icon_hash:
		return
	if not (res is PromiseError):
		_apply_custom_icon(btn, res.texture)


## Show the custom icon on the button's dedicated overlay node and blank the native glyph so
## the custom icon replaces it (rather than superimposing). The native glyph is put back by
## _restore_button_default_style / _apply_jump_icon on clear.
func _apply_custom_icon(btn: Button, texture: Texture2D) -> void:
	if btn.has_method("set_custom_icon"):
		btn.set_custom_icon(texture)
	btn.icon = null
	btn.text = ""


## Hide any custom-icon overlay and restore the button's captured default glyph/text (the
## latter only matters for the central button, whose glyph is borrowed on main_action swap).
func _restore_button_default_style(btn: Button, action: String) -> void:
	btn.set_meta("tc_icon_hash", "")
	if btn.has_method("clear_custom_icon"):
		btn.clear_custom_icon()
	btn.icon = _tc_default_icons.get(action)
	btn.text = _tc_default_text.get(action, "")
