extends Control

# Per-action glyphs (normal / pressed; glide also has a hold variant). Swapped by button state
# via each button's OrbSkin. Icons are 100x100, sized to the button through expand_icon.
const IC_INTERACT_NORMAL = preload("uid://c55dfgqwdxs8f")
const IC_INTERACT_PRESSED = preload("uid://ct0wqa804vtni")
const IC_E_NORMAL = preload("uid://ck3e0eaelc3rq")
const IC_E_PRESSED = preload("uid://01qlcj0sqqnw")
const IC_F_NORMAL = preload("uid://72h2xkpj1hgk")
const IC_F_PRESSED = preload("uid://c5u8stl6jg8cl")
const IC_1_NORMAL = preload("uid://e0ug4dbj1y10")
const IC_1_PRESSED = preload("uid://ddrk8qdneg8lw")
const IC_2_NORMAL = preload("uid://cqxtpai3pix5u")
const IC_2_PRESSED = preload("uid://bbqcb676u1mrv")
const IC_3_NORMAL = preload("uid://sw0euo71n3gv")
const IC_3_PRESSED = preload("uid://dm5mjc6eto6v1")
const IC_4_NORMAL = preload("uid://dvpirmcnk4c2a")
const IC_4_PRESSED = preload("uid://dx8f2nledowsj")
const IC_JUMP_NORMAL = preload("uid://d4neuk8df8m4y")
const IC_JUMP_PRESSED = preload("uid://dykud4ptnkdei")
const IC_DJUMP_NORMAL = preload("uid://df6fla5fsgl2s")
const IC_DJUMP_PRESSED = preload("uid://dmosa0apyje0c")
const IC_GLIDE_NORMAL = preload("uid://baojgdd2swsg1")
const IC_GLIDE_PRESSED = preload("uid://c52srwv13315")
const IC_GLIDE_HOLD = preload("uid://dio6yg4glplys")
const IC_PLUS_NORMAL = preload("uid://b14xrn24tfgpr")
const IC_PLUS_PRESSED = preload("uid://npyym5xkix66")

# action -> [normal, pressed]. Jump is dynamic (see _apply_jump_icon); pointer maps to interact.
const ACTION_ICONS := {
	"ia_pointer": [IC_INTERACT_NORMAL, IC_INTERACT_PRESSED],
	"ia_primary": [IC_E_NORMAL, IC_E_PRESSED],
	"ia_secondary": [IC_F_NORMAL, IC_F_PRESSED],
	"ia_action_3": [IC_1_NORMAL, IC_1_PRESSED],
	"ia_action_4": [IC_2_NORMAL, IC_2_PRESSED],
	"ia_action_5": [IC_3_NORMAL, IC_3_PRESSED],
	"ia_action_6": [IC_4_NORMAL, IC_4_PRESSED],
}

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

# Overflow actions hidden in the DEFAULT layout, so the "+" combo shows 2 buttons out of the
# box instead of 4 (HUD 2nd iteration). This is a joypad-presentation default that diverges from
# the SDK "unlisted = shown" rule for these two only: a scene re-enables either by listing it
# with hide=false in its PBTouchScreenControls.touch_inputs (see _apply_touch_controls). The
# buttons still exist, so a scene can surface up to 4 again.
const DEFAULT_HIDDEN: Dictionary = {
	"ia_action_5": true,
	"ia_action_6": true,
}

# Adaptive gamepad arc. The arc around the main button is, clockwise: the visible satellites
# followed by the "+" overflow toggle as the LAST / topmost element. They reflow together
# based on the number of visible arc elements, so the "+" always lands at the top when shown.
# LAYOUTS: key = number of visible arc elements, value = positions (top-left, relative to the
# Satellites anchor), slot 0 = lower-left (~9 o'clock) up to the last slot = top (~12).
# Design source (Figma file skocZRe2lV9IjqV4rF6EYs): N=4 is the full arc (3 satellites + "+"),
# from the "RightSideControls" HUD frame; the satellite-only counts come from the per-count
# frames (3 -> 5:1141, 2 -> 3:1187, 1 -> 3:1274). The N=4 entry mirrors the authored offsets of
# the arc nodes in joypad.tscn (Button_Interact / Button_Primary / Button_Secondary / Button_Combo);
# keep the two in sync when nudging the default arc in the editor.
const LAYOUTS := {
	1: [Vector2(-226, -62)],
	2: [Vector2(-226, -62), Vector2(-162, -200)],
	3: [Vector2(-229, -57), Vector2(-188, -175), Vector2(-57, -219)],
	4:
	[
		Vector2(-305, -98),
		Vector2(-296, -209),
		Vector2(-218, -290),
		Vector2(-97, -304),
	],
}

var combo_opened: bool = false

var _current_icon: int = ICON_DOUBLE_JUMP
# True while the glider is actively providing lift (latches the button's Hold look).
var _gliding: bool = false

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
	# Editor-only alignment guide (a centered disc to line the buttons up against); it lives in
	# the scene purely as a visual check and must never render at runtime.
	var align_guide: Control = get_node_or_null("%TextureRect_ToAlign")
	if align_guide != null:
		align_guide.visible = false

	for btn in _combo_action_buttons:
		btn.touch_action_changed.connect(_on_combo_action_changed)
	# Toggle the combo menu from raw touch so it opens with a second finger too
	# (Godot's emulated mouse only covers the primary touch). button_mask = 0 keeps
	# a single, consistent input path.
	button_combo.button_mask = 0
	button_combo.gui_input.connect(_on_button_combo_gui_input)
	# The "+" is a raw toggle (not a rendered action): skin its icon once. While the overflow
	# menu is open it latches into Hold (purple orb + plus-pressed), like the glider.
	_set_slot_icons(button_combo, IC_PLUS_NORMAL, IC_PLUS_PRESSED, IC_PLUS_PRESSED, false)
	_set_attenuated_sound_for_buttons(self)
	_apply_jump_icon(ICON_DOUBLE_JUMP)

	_pointer_btn = $Button_Press/Control/Button_Interact
	_primary_btn = $Button_Press/Control/Button_Primary
	_secondary_btn = $Button_Press/Control/Button_Secondary
	_quaternary_btn = $Button_Press/Control/Button_Quaternary

	_big_slot = button_press
	# Four arc slots. The 4th holds a real button when exactly 5 are visible (no "+"), and is
	# hidden in favor of the "+" toggle when the overflow menu is needed.
	_arc_slots = [_pointer_btn, _primary_btn, _secondary_btn, _quaternary_btn]
	_combo_slots = [%Button_Combo1, %Button_Combo2, %Button_Combo3, %Button_Combo4]

	_layout(DEFAULT_HIDDEN, {}, "")


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
		if _gliding:
			_gliding = false
			button_press.set_hold(false)
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
	if glide_active != _gliding:
		_gliding = glide_active
		button_press.set_hold(glide_active)


func _apply_jump_icon(icon_id: int) -> void:
	_current_icon = icon_id
	match icon_id:
		ICON_GLIDER:
			_set_slot_icons(button_press, IC_GLIDE_NORMAL, IC_GLIDE_PRESSED, IC_GLIDE_HOLD, true)
		ICON_SINGLE_JUMP:
			_set_slot_icons(button_press, IC_JUMP_NORMAL, IC_JUMP_PRESSED, null, true)
		_:
			_set_slot_icons(button_press, IC_DJUMP_NORMAL, IC_DJUMP_PRESSED, null, true)


## Finds the OrbSkin child that skins a button (null if the button isn't orb-skinned).
func _orb(node: Node) -> OrbSkin:
	for child in node.get_children():
		if child is OrbSkin:
			return child
	return null


## Assigns the per-state glyphs to a button's OrbSkin and sizes the icon (100 big / 56 small).
func _set_slot_icons(
	node: Button, normal: Texture2D, pressed: Texture2D, hold: Texture2D, is_big: bool
) -> void:
	var orb := _orb(node)
	if orb != null:
		orb.set_icons(normal, pressed, hold)
	node.text = ""
	node.expand_icon = true
	node.add_theme_constant_override("icon_max_width", 100 if is_big else 56)


func _set_attenuated_sound_for_buttons(node: Node) -> void:
	if node is Button:
		node.set_meta("attenuated_sound", true)

	for child in node.get_children():
		_set_attenuated_sound_for_buttons(child)


func _notification(what: int) -> void:
	# Collapse the "+" overflow menu whenever the joypad is hidden (by any flow), so it
	# never comes back open the next time the joypad is shown.
	if what == NOTIFICATION_VISIBILITY_CHANGED and not is_visible_in_tree():
		_close_combo()


func _on_button_combo_toggled(toggled_on: bool) -> void:
	combo_opened = toggled_on
	_set_combo_hold(toggled_on)
	if _combo_column:
		_combo_column.visible = toggled_on


## Latches the "+" orb into Hold while the overflow menu is open (like the glider).
func _set_combo_hold(on: bool) -> void:
	var orb := _orb(button_combo)
	if orb != null:
		orb.set_hold(on)


## Collapse the "+" overflow menu and reset the toggle without re-emitting `toggled`.
func _close_combo() -> void:
	if not combo_opened:
		return
	combo_opened = false
	button_combo.set_pressed_no_signal(false)
	_set_combo_hold(false)
	if _combo_column:
		_combo_column.visible = false


func _on_combo_action_changed(pressed: bool) -> void:
	if not pressed and combo_opened:
		# Clear the toggle BEFORE emitting `toggled(false)`: the handler refreshes the "+"
		# OrbSkin, which reads button_pressed — clearing it first lets the orb fall back to
		# Default instead of sticking on Pressed (set_pressed_no_signal fires no refresh).
		button_combo.set_pressed_no_signal(false)
		button_combo.toggled.emit(false)


## Applies PBTouchScreenControls (Global.touch_controls_*). No component (inactive) → the
## default priority-stack layout (with DEFAULT_HIDDEN applied, so the combo shows 2). Active →
## the stack is recomputed from DEFAULT_HIDDEN plus the scene's `hide` denylist, with a listed
## `hide=false` re-showing a default-hidden action, and the `main_action` override (see _layout).
## The joypad governs its own visibility (shown on desktop too), so this runs regardless of platform.
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
			_layout(DEFAULT_HIDDEN, {}, "")
		return

	_tc_active_applied = true

	var hidden: Dictionary = DEFAULT_HIDDEN.duplicate()
	var icons := {}  # action -> { "hash": String, "url": String }
	for entry in inputs:
		var action := String(entry.get("action", ""))
		if bool(entry.get("hide", false)):
			hidden[action] = true
		else:
			# Explicit show: a listed hide=false re-enables a default-hidden action (opt-in).
			hidden.erase(action)
		var custom_icon := SdkTouchControlsApplier.get_custom_icon_for_action(action)
		if not custom_icon.is_empty():
			icons[action] = custom_icon

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
	# the buttons as their count changes. The 32 px lifts the first (bottom) combo button that
	# far above the "+"; the inter-button gap is the column's `separation` (design: 20 px).
	if _combo_column:
		var col_size := _combo_column.get_combined_minimum_size()
		_combo_column.position = Vector2(
			(button_combo.size.x - col_size.x) / 2.0, -col_size.y - 32.0
		)
		_combo_column.size = col_size
		_combo_column.visible = combo_opened and show_plus

	_jump_slot_is_big = n > 0 and visible[0] == "ia_jump"


## Renders `action` onto physical `node`: sets its trigger_action, clears any prior occupant's
## visuals, then applies either a scene icon override (`icon` = { hash, url }) or the action's
## normal/pressed glyph pair (via the button's OrbSkin). `is_big` sizes the icon (100 big /
## 56 small) and selects the dynamic jump logic (big central button only).
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
		_apply_jump_icon(_current_icon)
		return

	# Jump demoted to a satellite: static double/single-jump glyph, no glider / hold.
	if action == "ia_jump":
		var dj_off := Global.is_double_jump_disabled()
		var jump_normal := IC_JUMP_NORMAL if dj_off else IC_DJUMP_NORMAL
		var jump_pressed := IC_JUMP_PRESSED if dj_off else IC_DJUMP_PRESSED
		_set_slot_icons(node, jump_normal, jump_pressed, null, is_big)
		return

	# Mapped actions (pointer / E / F / 1-4) → their orb-swapped normal+pressed icon pair.
	# Every action in PRIORITY_ORDER is covered above; an unmapped action just keeps the
	# cleared (glyph-less) orb from _reset_node_visuals.
	if ACTION_ICONS.has(action):
		var pair: Array = ACTION_ICONS[action]
		_set_slot_icons(node, pair[0], pair[1], null, is_big)


## Clears any prior occupant's presentation so a repurposed slot starts from a clean state.
func _reset_node_visuals(node: Button) -> void:
	node.set_meta("tc_icon_hash", "")
	if node.has_method("clear_custom_icon"):
		node.clear_custom_icon()
	if node.has_method("set_hold"):
		node.set_hold(false)
	var orb := _orb(node)
	if orb != null:
		orb.set_icons(null, null, null)
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
