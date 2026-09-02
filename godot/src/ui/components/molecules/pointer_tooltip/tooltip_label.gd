extends PanelContainer

const BG_COLOR_NORMAL: String = "#00000080"
const BG_COLOR_PRESSED: String = "#44444480"
const ICON_LEFT_CLICK = preload("uid://cljfaeb8np0ma")
const ICON_INTERACTIVE_POINTER = preload("uid://72xpjysoxgwo")
const ICON_JUMP = preload("uid://ck3atqpytstpo")

const MOBILE_ACTION_MAP := {
	"ia_pointer": [ICON_INTERACTIVE_POINTER, "Tap"],
	"ia_jump": [ICON_JUMP, "Jump"],
	"ia_primary": ["E", "Primary"],
	"ia_secondary": ["F", "Secondary"],
	"ia_action_3": ["1", "Action 1"],
	"ia_action_4": ["2", "Action 2"],
	"ia_action_5": ["3", "Action 3"],
	"ia_action_6": ["4", "Action 4"],
}

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
@onready var panel_container_inputs: PanelContainer = %PanelContainer_Inputs
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


func set_tooltip_data(text_pet_down: String, text_pet_up, action: String):
	text_down = text_pet_down if !text_pet_down.is_empty() else text_pet_up
	text_up = text_pet_up if !text_pet_up.is_empty() else text_pet_down

	var action_lower: String = action.to_lower()
	# Reset any pending custom-icon load from a previous action on this reused node.
	_custom_icon_hash = ""

	if not label_text:
		return

	if Global.is_mobile() and action_lower in MOBILE_ACTION_MAP:
		var mapping: Array = MOBILE_ACTION_MAP[action_lower]
		var mobile_label: String = mapping[1]
		if mapping[0] is Texture2D:
			_show_keyboard_icon(mapping[0])
		else:
			_show_keyboard(mapping[0])
		action_to_trigger = action_lower
		if text_down.is_empty():
			text_down = mobile_label
		if text_up.is_empty():
			text_up = mobile_label
		label_text.text = text_down
	elif action_lower == "ia_any":
		_show_keyboard("Any")
		action_to_trigger = action_lower
		label_text.text = text_down
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
					key = ICON_INTERACTIVE_POINTER if Global.is_mobile() else ICON_LEFT_CLICK
		else:
			key = ICON_INTERACTIVE_POINTER

		if key != null:
			if key is String:
				_show_keyboard(key)
			else:
				_show_keyboard_icon(key)
			action_to_trigger = action_lower
			label_text.text = text_down
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
		_show_keyboard_icon(cached)
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
		_show_keyboard_icon(res.texture)


func _show_keyboard(text: String) -> void:
	show()
	panel_container_inputs.show()
	label_action.show()
	texture_rect_action_icon.hide()
	label_action.text = text


func _show_keyboard_icon(icon: Texture2D) -> void:
	show()
	panel_container_inputs.show()
	texture_rect_action_icon.show()
	label_action.hide()
	texture_rect_action_icon.texture = icon


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
		label_text.text = text_up if new_pressed else text_down
		last_state_pressed = new_pressed


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
