extends Control

const GLIDER_ICON_MAX_WIDTH = 85
const GLIDER_ICON = preload("uid://dnosnq2stqu11")  # "res://assets/themes/dark_dcl_theme/icons/Glider.svg"

const DOUBLE_JUMP_ICON_MAX_WIDTH = 52
const DOUBLE_JUMP_ICON = preload("uid://euvimxirt85b")  # "res://assets/themes/dark_dcl_theme/icons/DoubleJump.svg"

const SINGLE_JUMP_ICON_MAX_WIDTH = 85
const SINGLE_JUMP_ICON = preload("uid://ck3atqpytstpo")  # "res://assets/themes/dark_dcl_theme/icons/Jump.svg"

# Pointer glyph has two hand-tuned variants (Figma nodes 3:869 small / 3:1008 large) so the
# outline weight stays visually constant instead of scaling with the button. The small variant
# is authored on the satellite (joypad.tscn); the large one is swapped onto the central button
# when the pointer is promoted to the main action. See _apply_main_action.
const POINTER_SMALL_ICON = preload("uid://cio1wsarij08p")  # pointer_small.svg (node 3:869)
const POINTER_LARGE_ICON = preload("uid://5nfo22llcuuv")  # pointer_large.svg (node 3:1008)
const POINTER_LARGE_ICON_MAX_WIDTH = 55  # 54/118 of the Figma big button, on the 120px center

# Button SKIN (border/fill/radius/colors) is owned by the theme, not this organism: the
# `TouchableButton` variation in assets/themes/dcl_theme.tres -> touchable_normal.tres. The
# joypad only swaps these two styleboxes onto the central button for the glider inverted state.
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

# Single priority stack (issue #2518): all on-screen buttons form one ordered list and fill
# fixed screen positions from the top down. Hiding an action cascades the rest up; the first
# visible action is the big central button; a scene main_action moves its action to the front.
const PRIORITY_ORDER := [
	"ia_jump",
	"ia_pointer",
	"ia_primary",
	"ia_secondary",
	"ia_action_3",
	"ia_action_4",
	"ia_action_5",
	"ia_action_6",
]

# Adaptive gamepad arc. The arc around the main button is, clockwise: the visible satellites
# followed by the "+" overflow toggle as the LAST / topmost element. They reflow together
# based on the number of visible arc elements, so the "+" always lands at the top when shown.
# LAYOUTS: key = number of visible arc elements, value = positions (top-left, relative to the
# Satellites anchor), slot 0 = lower-left (~9 o'clock) up to the last slot = top (~12).
# Design source (Figma file skocZRe2lV9IjqV4rF6EYs): N=4 is the full arc (3 satellites + "+"),
# from the "RightSideControls" HUD frame; the satellite-only counts come from the per-count
# frames (3 -> 5:1141, 2 -> 3:1187, 1 -> 3:1274).
const LAYOUTS := {
	1: [Vector2(-226, -62)],
	2: [Vector2(-226, -62), Vector2(-162, -200)],
	3: [Vector2(-229, -57), Vector2(-188, -175), Vector2(-57, -219)],
	4: [Vector2(-229, -57), Vector2(-208, -145), Vector2(-140, -202), Vector2(-57, -219)],
}

# Jump rendered on a small satellite (when demoted from the center) uses a static glyph sized
# for the 65px satellite, instead of the dynamic/glider logic reserved for the big button.
const JUMP_SATELLITE_ICON_MAX_WIDTH := 40

var combo_opened: bool = false

var _current_icon: int = ICON_DOUBLE_JUMP
var _showing_inverted_colors: bool = false

# PBTouchScreenControls (Global.touch_controls_*): the joypad renders any action on any
# physical slot each layout pass (see _layout). `_action_defaults` snapshots each action's
# authored glyph (icon / text / theme overrides), keyed by action so a slot can render
# whatever action the priority stack assigns to it. Populated in _ready.
var _action_defaults: Dictionary = {}
var _tc_active_applied: bool = false
var _last_tc_hash: int = 0
var _jump_icon_overridden: bool = false
var _jump_slot_is_big: bool = true

# Physical slots (see joypad.tscn): the big central button, the three arc satellites (in
# fixed screen order matching LAYOUTS slots 0..2), and the four overflow-column buttons
# (priority order, index 0 = first overflow). The "+" toggle is `button_combo` below.
var _big_slot: Button
var _arc_slots: Array[Button] = []
var _combo_slots: Array[Button] = []

# The satellite buttons (authored flat on the Satellites anchor, `Button_Press/Control`) and
# the vertical column that holds the combo buttons. All live in the scene now — see joypad.tscn.
var _pointer_btn: Button
var _primary_btn: Button
var _secondary_btn: Button
var _quaternary_btn: Button

@onready var button_combo: Button = %Button_Combo
@onready var button_press: Button = $Button_Press
@onready var _combo_column: VBoxContainer = %ComboColumn

@onready var _combo_action_buttons: Array[Button] = [
	%Button_Combo1,
	%Button_Combo2,
	%Button_Combo3,
	%Button_Combo4,
]


func _ready() -> void:
	for btn in _combo_action_buttons:
		btn.touch_action_changed.connect(_on_combo_action_changed)
	# Toggle the combo menu from raw touch so it opens with a second finger too
	# (Godot's emulated mouse only covers the primary touch). button_mask = 0 keeps
	# a single, consistent input path.
	button_combo.button_mask = 0
	button_combo.gui_input.connect(_on_button_combo_gui_input)
	_set_attenuated_sound_for_buttons(self)
	_apply_jump_icon(ICON_DOUBLE_JUMP)

	_pointer_btn = $Button_Press/Control/Button_Jump
	_primary_btn = $Button_Press/Control/Button_Primary
	_secondary_btn = $Button_Press/Control/Button_Secondary
	_quaternary_btn = $Button_Press/Control/Button_Quaternary

	_big_slot = button_press
	# Four arc slots. The 4th holds a real button when exactly 5 are visible (no "+"), and is
	# hidden in favor of the "+" toggle when the overflow menu is needed.
	_arc_slots = [_pointer_btn, _primary_btn, _secondary_btn, _quaternary_btn]
	_combo_slots = [%Button_Combo1, %Button_Combo2, %Button_Combo3, %Button_Combo4]

	# Snapshot each action's authored default presentation from its home node, so any slot can
	# later render any action's glyph. Keyed by action (independent of node).
	var home := {
		"ia_jump": button_press,
		"ia_pointer": _pointer_btn,
		"ia_primary": _primary_btn,
		"ia_secondary": _secondary_btn,
		"ia_action_3": %Button_Combo1,
		"ia_action_4": %Button_Combo2,
		"ia_action_5": %Button_Combo3,
		"ia_action_6": %Button_Combo4,
	}
	for action in home:
		var btn := home[action] as Button
		var imw := -1
		if btn.has_theme_constant_override("icon_max_width"):
			imw = btn.get_theme_constant("icon_max_width")
		var fs := -1
		if btn.has_theme_font_size_override("font_size"):
			fs = btn.get_theme_font_size("font_size")
		_action_defaults[action] = {
			"icon": btn.icon,
			"text": btn.text,
			"imw": imw,
			"fs": fs,
		}

	_layout({}, {}, "")


func _on_button_combo_gui_input(event: InputEvent) -> void:
	if event is InputEventScreenTouch and event.pressed:
		# Flipping button_pressed emits `toggled`, which opens/closes the combo menu.
		button_combo.button_pressed = not button_combo.button_pressed
		button_combo.accept_event()


func _process(_dt: float) -> void:
	_update_jump_icon()
	_apply_touch_controls()


func _update_jump_icon() -> void:
	# The central button's icon is dynamic only while jump occupies the big slot and is not
	# overridden by a scene icon. When jump is demoted / hidden, drop any inverted styling.
	if not _jump_slot_is_big or _jump_icon_overridden:
		_showing_inverted_colors = false
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
		_apply_inverted_colors(button_press, glide_active)


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


func _apply_inverted_colors(btn: Button, inverted: bool) -> void:
	if inverted:
		for style in INVERTED_NORMAL_STYLES:
			btn.add_theme_stylebox_override(style, TOUCHABLE_PRESSED_STYLEBOX)
		for style in INVERTED_PRESSED_STYLES:
			btn.add_theme_stylebox_override(style, TOUCHABLE_NORMAL_STYLEBOX)
		for color_name in INVERTED_NORMAL_ICON_COLORS:
			btn.add_theme_color_override(color_name, TOUCHABLE_ICON_DARK)
		for color_name in INVERTED_PRESSED_ICON_COLORS:
			btn.add_theme_color_override(color_name, TOUCHABLE_ICON_LIGHT)
	else:
		for style in INVERTED_NORMAL_STYLES:
			btn.remove_theme_stylebox_override(style)
		for style in INVERTED_PRESSED_STYLES:
			btn.remove_theme_stylebox_override(style)
		for color_name in INVERTED_NORMAL_ICON_COLORS:
			btn.remove_theme_color_override(color_name)
		for color_name in INVERTED_PRESSED_ICON_COLORS:
			btn.remove_theme_color_override(color_name)


func _set_attenuated_sound_for_buttons(node: Node) -> void:
	if node is Button:
		node.set_meta("attenuated_sound", true)

	for child in node.get_children():
		_set_attenuated_sound_for_buttons(child)


func _on_button_combo_toggled(toggled_on: bool) -> void:
	combo_opened = toggled_on
	if _combo_column:
		_combo_column.visible = toggled_on


func _on_combo_action_changed(pressed: bool) -> void:
	if not pressed and combo_opened:
		button_combo.toggled.emit(false)
		button_combo.set_pressed_no_signal(false)


## Applies PBTouchScreenControls (Global.touch_controls_*). No component (inactive) → the
## default priority-stack layout. Active → the stack is recomputed from the `hide` denylist
## and the `main_action` override (see _layout). The joypad governs its own visibility (shown
## on desktop too), so this runs regardless of platform.
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
			_layout({}, {}, "")
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

	_layout(hidden, icons, main_action)


## Recomputes the whole gamepad: builds the visible priority list (apply hide denylist, then
## move main_action to the front) and assigns those actions to the physical slots.
func _layout(hidden: Dictionary, icons: Dictionary, main_action: String) -> void:
	var visible := _compute_visible_list(hidden, main_action)
	_assign_slots(visible, icons)


## The priority stack minus hidden actions, with `main_action` (if visible and not jump)
## promoted to the front. Hide wins over main_action: a hidden main_action target is not
## resurrected (contradictory config is treated as a scene bug).
func _compute_visible_list(hidden: Dictionary, main_action: String) -> Array:
	var visible: Array = []
	for action in PRIORITY_ORDER:
		if not hidden.has(action):
			visible.append(action)
	if not main_action.is_empty() and main_action != "ia_jump":
		var idx := visible.find(main_action)
		if idx > 0:
			visible.remove_at(idx)
			visible.insert(0, main_action)
	return visible


## Assigns the visible actions to physical slots: index 0 → big central button; the next up to
## 3 → arc satellites; when more than 4 are visible the "+" takes the 4th arc slot and the
## remaining actions (indices 4+) go into the overflow column.
func _assign_slots(visible: Array, icons: Dictionary) -> void:
	var n := visible.size()
	# The arc has four slots. Without the "+" they all hold real buttons (big + 4 = 5 shown
	# directly); the "+" only appears when a 6th button would overflow, and it takes the 4th
	# arc slot (leaving big + 3 shown directly, the rest behind it).
	var show_plus := n > 5
	var arc_cap := 3 if show_plus else 4

	# Big central button = first visible action (nothing when everything is hidden).
	if n > 0:
		_big_slot.visible = true
		_render_action_on(_big_slot, visible[0], icons.get(visible[0], {}), true)
	else:
		_big_slot.visible = false

	# Arc satellites: the next actions after the big one, up to the arc capacity.
	var arc_actions: Array = []
	for i in range(1, n):
		if arc_actions.size() >= arc_cap:
			break
		arc_actions.append(visible[i])

	for i in range(_arc_slots.size()):
		var slot := _arc_slots[i]
		if i < arc_actions.size():
			slot.visible = true
			_render_action_on(slot, arc_actions[i], icons.get(arc_actions[i], {}), false)
		else:
			slot.visible = false

	# The "+" overflow toggle occupies the topmost arc slot only when there are extra buttons.
	button_combo.visible = show_plus

	# Position the arc (satellites first, "+" last) from LAYOUTS by the arc element count.
	var arc_nodes: Array = []
	for i in range(arc_actions.size()):
		arc_nodes.append(_arc_slots[i])
	if show_plus:
		arc_nodes.append(button_combo)
	var positions: Array = LAYOUTS.get(arc_nodes.size(), [])
	for i in range(arc_nodes.size()):
		if i < positions.size():
			arc_nodes[i].position = positions[i]

	# Overflow column (indices 4+). Rendered now but only shown while the "+" menu is open.
	var overflow: Array = []
	if show_plus:
		for i in range(4, n):
			overflow.append(visible[i])
	for i in range(_combo_slots.size()):
		var slot := _combo_slots[i]
		if i < overflow.size():
			slot.visible = true
			_render_action_on(slot, overflow[i], icons.get(overflow[i], {}), false)
		else:
			slot.visible = false

	# The combo column sits directly on top of the "+" toggle; collapsed unless open. Size it
	# to its visible content (min size) so the buttons pack tightly with no overlap and no
	# centering gap — the authored .tscn frame is a fixed box that would otherwise clip / float
	# the buttons as their count changes.
	if _combo_column:
		var col_size := _combo_column.get_combined_minimum_size()
		_combo_column.position = Vector2(
			(button_combo.size.x - col_size.x) / 2.0, -col_size.y - 8.0
		)
		_combo_column.size = col_size
		_combo_column.visible = combo_opened and show_plus

	_jump_slot_is_big = n > 0 and visible[0] == "ia_jump"


## Renders `action` onto physical `node`: sets its trigger_action, clears any prior occupant's
## visuals, then applies either a scene icon override (`icon` = { hash, url }) or the action's
## canonical glyph. `is_big` selects size-specific glyphs (pointer) and the dynamic jump logic
## (big central button only).
func _render_action_on(node: Button, action: String, icon: Dictionary, is_big: bool) -> void:
	node.trigger_action = action
	_reset_node_visuals(node)

	var icon_hash := String(icon.get("hash", ""))
	if not icon_hash.is_empty():
		if action == "ia_jump" and is_big:
			_jump_icon_overridden = true
		_async_set_button_icon(node, icon_hash, String(icon.get("url", "")))
		return

	# Jump on the big central button keeps its dynamic glider / double-jump behavior.
	if action == "ia_jump" and is_big:
		_jump_icon_overridden = false
		node.text = ""
		_apply_jump_icon(_current_icon)
		return

	var d: Dictionary = _action_defaults.get(action, {})
	var glyph_icon: Texture2D = d.get("icon")
	var glyph_text := String(d.get("text", ""))
	var imw: int = d.get("imw", -1)
	var fs: int = d.get("fs", -1)

	if action == "ia_pointer":
		# Pointer uses a size-specific variant so the outline weight matches the design.
		glyph_text = ""
		if is_big:
			glyph_icon = POINTER_LARGE_ICON
			imw = POINTER_LARGE_ICON_MAX_WIDTH
		else:
			glyph_icon = POINTER_SMALL_ICON
	elif action == "ia_jump":
		# Jump demoted to a satellite: static glyph, no glider / inverted styling.
		glyph_text = ""
		glyph_icon = SINGLE_JUMP_ICON if Global.is_double_jump_disabled() else DOUBLE_JUMP_ICON
		imw = JUMP_SATELLITE_ICON_MAX_WIDTH

	node.icon = glyph_icon
	node.text = glyph_text
	if imw >= 0:
		node.add_theme_constant_override("icon_max_width", imw)
	if fs >= 0:
		node.add_theme_font_size_override("font_size", fs)


## Clears any prior occupant's presentation so a repurposed slot starts from a clean state.
func _reset_node_visuals(node: Button) -> void:
	node.set_meta("tc_icon_hash", "")
	if node.has_method("clear_custom_icon"):
		node.clear_custom_icon()
	_apply_inverted_colors(node, false)
	node.icon = null
	node.text = ""
	node.remove_theme_constant_override("icon_max_width")
	node.remove_theme_font_size_override("font_size")


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
