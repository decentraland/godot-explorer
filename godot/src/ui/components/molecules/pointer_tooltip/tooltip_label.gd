class_name TooltipLabel
extends PanelContainer

## The avatar "View profile" prompt, emitted as a KEY by scene_manager.rs rather than as English
## prose. Label_Text is auto_translate_mode = 2 because it normally carries creator-authored
## PointerEvents text, which must never be looked up — so only this one known marker is resolved
## here, never the scene-supplied strings around it. explorer.gd matches on the same constant to
## filter the prompt for the "Hide View Profile" setting.
# i18n-keys: TOOLTIP_VIEW_PROFILE
const VIEW_PROFILE_KEY: String = "TOOLTIP_VIEW_PROFILE"

# Set on the stylebox in _ready, which overwrites whatever tooltip_label.tscn declares -- the
# scene's bg_color never renders, so change it HERE. #161518 at 70% is the pill fill from the
# HUD-Revamp "Interaction prompt" frame. The pressed tint is tap feedback the design does not
# specify.
const BG_COLOR_NORMAL: String = "#161518B3"
const BG_COLOR_PRESSED: String = "#444348B3"
## Shared HUD glyph tint (Figma "IconHUD"), used to tint the built-in monochrome-white icons;
## scene-replaced creator icons keep their own colors (see _show_keyboard_icon). The keyboard
## letter is the same color but is not driven from here -- it comes from LabelSettings_r3qmc in
## tooltip_label.tscn, so change both together.
const ICON_COLOR := Color("#DFD0FF")
## Invisible tap growth per side. Same _has_point trick as the TapArea atom
## (src/ui/components/atoms/tap_area/tap_area.gd), which this node can't extend because it is a
## PanelContainer. The design asks for 10px, but that is 15 device px -- under a millimetre -- once
## content_scale_factor is applied on a phone, so it is widened here to reach the ~48dp minimum
## touch target. Vertical growth is dropped when tooltips are stacked (see tap_grow_y).
const TAP_GROW_X: float = 24.0
const TAP_GROW_Y: float = 24.0
# Desktop-only: the left-click glyph has no joypad counterpart, so it is not in ActionIcons.
const ICON_LEFT_CLICK = preload("uid://cljfaeb8np0ma")
## Widest the label may grow before the text is elided. This must be applied as a clamp in code:
## with text_overrun_behavior set, a Label's minimum width collapses to ~0, so putting the cap in
## custom_minimum_size makes it a fixed width and every pill ends up this wide.
const LABEL_MAX_WIDTH: float = 313.0

## Vertical half of the tap growth. Stacked tooltips clear each other by only PILL_GAP (8px),
## so growing them vertically would make neighbouring hit areas overlap and let a tap fire the
## wrong prompt. pointer_tooltip.gd zeroes this whenever more than one tooltip is on screen.
var tap_grow_y: float = TAP_GROW_Y
var action_to_trigger: String = ""
var text_down := ""
var text_up := ""
var last_state_pressed := false
var stylebox: StyleBox
# Content hash of the scene-replaced (PBTouchScreenControls) icon we're currently loading,
# so a stale async fetch can bail if the tooltip is reused for another action meanwhile.
var _custom_icon_hash: String = ""

@onready var label_action = %Label_Action
@onready var texture_rect_action_icon = %TextureRect_ActionIcon
@onready var label_text = %Label_Text
@onready var margin_container_icons: MarginContainer = %MarginContainer_Icons


func _ready():
	stylebox = self.get_theme_stylebox("panel").duplicate()
	add_theme_stylebox_override("panel", stylebox)

	set_bg_color(BG_COLOR_NORMAL)
	if Global.is_mobile():
		self.gui_input.connect(self.mobile_on_panel_container_gui_input)


func set_bg_color(color):
	stylebox.bg_color = color


## Resolve only our own prompts. Anything else is creator copy and passes through untouched.
static func _resolve(value: String) -> String:
	return TranslationServer.translate(value) if value == VIEW_PROFILE_KEY else value


func set_tooltip_data(text_pet_down: String, text_pet_up, action: String):
	text_down = _resolve(text_pet_down if !text_pet_down.is_empty() else text_pet_up)
	text_up = _resolve(text_pet_up if !text_pet_up.is_empty() else text_pet_down)

	var action_lower: String = action.to_lower()
	# Reset any pending custom-icon load from a previous action on this reused node.
	_custom_icon_hash = ""

	if not label_text:
		return

	if Global.is_mobile() and ActionIcons.has_action(action_lower):
		# Same table the joypad skins its buttons from, so the hint always matches the button.
		# The label is a translation KEY: Label_Text is auto_translate_mode = 2 (it normally
		# carries creator-authored PointerEvents text), so it is resolved here, not looked up.
		var mobile_label: String = ActionIcons.tooltip_label_key(action_lower).text()
		var glyph: Texture2D = ActionIcons.tooltip_icon(action_lower)
		if glyph != null:
			_show_keyboard_icon(glyph)
		else:
			_show_keyboard(ActionIcons.tooltip_keycap(action_lower))
		action_to_trigger = action_lower
		if text_down.is_empty():
			text_down = mobile_label
		if text_up.is_empty():
			text_up = mobile_label
		_set_label_text(text_down)
	elif action_lower == "ia_any":
		_show_keyboard("Any")
		action_to_trigger = action_lower
		_set_label_text(text_down)
	else:
		var index: int = InputMap.get_actions().find(action_lower, 0)
		if index == -1:
			hide()
			action_to_trigger = ""
			printerr("Action doesn't exist ", action)
			return

		var key: Variant = null
		var action_events: Array[InputEvent] = InputMap.action_get_events(
			InputMap.get_actions()[index]
		)
		if !action_events.is_empty():
			var event = action_events[0]
			if event is InputEventKey:
				key = char(event.unicode).to_upper()
			elif event is InputEventMouseButton:
				if event.button_index == 1:
					key = (
						ActionIcons.tooltip_icon("ia_pointer")
						if Global.is_mobile()
						else ICON_LEFT_CLICK
					)
		else:
			key = ActionIcons.tooltip_icon("ia_pointer")

		if key != null:
			if key is String:
				_show_keyboard(key)
			else:
				_show_keyboard_icon(key)
			action_to_trigger = action_lower
			_set_label_text(text_down)
		else:
			hide()
			action_to_trigger = ""
			printerr("Action doesn't exist ", action)

	# If a scene replaced this action's icon (PBTouchScreenControls), swap the default glyph
	# for that same icon so the hint matches the on-screen gamepad button. The default stays
	# visible until the texture finishes loading (and if it never does).
	if action_to_trigger == action_lower:
		_async_apply_custom_icon_override(action_lower)


## Loads the scene-replaced icon for `action` (shared with the joypad via
## SdkTouchControlsApplier) and shows it in the icon slot once ready. Mirrors joypad.gd's
## _async_set_button_icon: sync cache hit applies immediately, otherwise awaits the fetch,
## bailing if this node was reassigned to another action meanwhile.
func _async_apply_custom_icon_override(action: String) -> void:
	var custom_icon := SdkTouchControlsApplier.get_custom_icon_for_action(action)
	if custom_icon.is_empty():
		return

	var icon_hash := String(custom_icon.get("hash", ""))
	var icon_url := String(custom_icon.get("url", ""))
	var scene_id := int(custom_icon.get("scene_id", -1))
	_custom_icon_hash = icon_hash

	var cached: Texture2D = Global.content_provider.get_texture_from_hash(icon_hash)
	if cached != null:
		_show_keyboard_icon(cached, Color.WHITE)
		return

	# Fetch by hash through the declaring scene's mapping so this shares the cache slot with
	# the scene's own UI and gets purged on a preview hot-reload (#2796); the by-URL path is
	# the fallback for an already-unloaded scene. Same rationale as joypad.gd.
	var mapping: DclContentMappingAndUrl = Global.scene_runner.get_scene_content_mapping(scene_id)
	var promise: Promise
	if mapping.get_base_url().is_empty():
		promise = Global.content_provider.fetch_texture_by_url(icon_hash, icon_url)
	else:
		promise = Global.content_provider.fetch_texture_by_hash(icon_hash, mapping)
	var res = await PromiseUtils.async_awaiter(promise)
	# Bail if the tooltip was reused for another action while we were awaiting.
	if _custom_icon_hash != icon_hash:
		return
	if not (res is PromiseError):
		_show_keyboard_icon(res.texture, Color.WHITE)


func _show_keyboard(text: String) -> void:
	show()
	label_action.show()
	texture_rect_action_icon.hide()
	label_action.text = text


## `tint` is ICON_COLOR for the built-in white glyphs, but Color.WHITE (i.e. untinted) for a
## scene-replaced creator icon, which arrives with its own colors — same as the on-screen gamepad
## button renders it (button_touch_action.tscn's CustomIcon has no modulate).
func _show_keyboard_icon(icon: Texture2D, tint: Color = ICON_COLOR) -> void:
	show()
	texture_rect_action_icon.show()
	label_action.hide()
	texture_rect_action_icon.self_modulate = tint
	texture_rect_action_icon.texture = icon


## Grow the touch target without changing the visual size.
func _has_point(point: Vector2) -> bool:
	var rect := Rect2(Vector2.ZERO, size)
	return rect.grow_individual(TAP_GROW_X, tap_grow_y, TAP_GROW_X, tap_grow_y).has_point(point)


func _physics_process(_delta):
	if action_to_trigger == "ia_any":
		return

	# React to live icon changes (e.g. the Controls Builder reassigning a glyph) while the
	# tooltip stays on screen: if this action's scene-replaced icon changed (added, swapped,
	# or removed), re-resolve the glyph. Cheap no-op when the hash is unchanged.
	if not action_to_trigger.is_empty():
		var desired_hash := String(
			SdkTouchControlsApplier.get_custom_icon_for_action(action_to_trigger).get("hash", "")
		)
		if desired_hash != _custom_icon_hash:
			set_tooltip_data(text_down, text_up, action_to_trigger)

	var new_pressed = Input.is_action_pressed(action_to_trigger)
	if last_state_pressed != new_pressed:
		set_bg_color(BG_COLOR_PRESSED if new_pressed else BG_COLOR_NORMAL)
		margin_container_icons.add_theme_constant_override("margin_top", 2 if new_pressed else 0)
		_set_label_text(text_up if new_pressed else text_down)
		last_state_pressed = new_pressed


func _set_label_text(value: String) -> void:
	label_text.text = value
	var settings: LabelSettings = label_text.label_settings
	var font: Font = (
		settings.font if settings and settings.font else label_text.get_theme_font("font")
	)
	var font_size: int = (
		settings.font_size if settings else label_text.get_theme_font_size("font_size")
	)
	var text_width: float = font.get_string_size(value, HORIZONTAL_ALIGNMENT_LEFT, -1, font_size).x
	label_text.custom_minimum_size.x = minf(ceilf(text_width), LABEL_MAX_WIDTH)


func mobile_on_panel_container_gui_input(event):
	if event is InputEventScreenTouch:
		# ia_any is a virtual action with no InputMap entry — pressing it
		# would emit "InputMap action 'ia_any' doesn't exist" engine errors.
		if action_to_trigger.is_empty() or action_to_trigger == "ia_any":
			return
		if event.pressed:
			Input.action_press(action_to_trigger)
		else:
			Input.action_release(action_to_trigger)
