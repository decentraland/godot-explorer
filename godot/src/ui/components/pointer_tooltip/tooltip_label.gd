extends PanelContainer

const BG_COLOR_NORMAL: String = "#00000080"
const BG_COLOR_PRESSED: String = "#44444480"
const ICON_LEFT_CLICK = preload("uid://cljfaeb8np0ma")
const ICON_INTERACTIVE_POINTER = preload("uid://72xpjysoxgwo")
const A_FILLED = preload("uid://bjppxn35ls1rc")
const B_FILLED = preload("uid://c4f72aulqlr71")
const X_FILLED = preload("uid://c6ee63ym35sw5")
const Y_FILLED = preload("uid://c57fjtwt0mnfd")
const LEFT_JOYSTICK_DOWN = preload("uid://pys3tuxq441v")
const LEFT_JOYSTICK_LEFT = preload("uid://dhbpp6aai8mus")
const LEFT_JOYSTICK_RIGHT = preload("uid://ckmdvpdo4eye6")
const LEFT_JOYSTICK_UP = preload("uid://dhsdl8kfl4xmu")
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

const GAMEPAD_BUTTON_MAP := {
	"ia_jump": [false, A_FILLED, "Press Jump"],
	"ia_primary": [false, B_FILLED, "Press Primary"],
	"ia_pointer": [false, X_FILLED, "Press Interact"],
	"ia_secondary": [false, Y_FILLED, "Press Secondary"],
	"ia_action_3": [true, A_FILLED, "Press Action 1"],
	"ia_action_4": [true, B_FILLED, "Press Action 2"],
	"ia_action_5": [true, X_FILLED, "Press Action 3"],
	"ia_action_6": [true, Y_FILLED, "Press Action 4"],
	"ia_forward": [false, LEFT_JOYSTICK_UP, "Move the stick to the Forward"],
	"ia_backward": [false, LEFT_JOYSTICK_DOWN, "Move the stick to the Backward"],
	"ia_left": [false, LEFT_JOYSTICK_LEFT, "Move the stick to the Left"],
	"ia_right": [false, LEFT_JOYSTICK_RIGHT, "Move the stick to the Right"],
}

var action_to_trigger: String = ""
var text_down := ""
var text_up := ""
var last_state_pressed := false
var stylebox: StyleBox

@onready var label_action = %Label_Action
@onready var texture_rect_action_icon = %TextureRect_ActionIcon
@onready var h_box_container_gamepad: HBoxContainer = %HBoxContainer_Gamepad
@onready var panel_container_inputs: PanelContainer = %PanelContainer_Inputs
@onready var h_box_container_modifier: HBoxContainer = %HBoxContainer_Modifier
@onready var texture_rect_gamepad_button: TextureRect = %TextureRect_GamepadButton
@onready var label_text = %Label_Text


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
	var gamepad_connected := Input.get_connected_joypads().size() > 0

	if not label_text:
		return

	if gamepad_connected and action_lower in GAMEPAD_BUTTON_MAP:
		var mapping: Array = GAMEPAD_BUTTON_MAP[action_lower]
		_show_gamepad(mapping[0], mapping[1])
		action_to_trigger = action_lower
		var gamepad_label: String = mapping[2]
		if text_down.is_empty():
			text_down = gamepad_label
		if text_up.is_empty():
			text_up = gamepad_label
		label_text.text = text_down
	elif Global.is_mobile() and action_lower in MOBILE_ACTION_MAP:
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


func _show_gamepad(has_modifier: bool, button_texture: Texture2D) -> void:
	show()
	h_box_container_gamepad.show()
	panel_container_inputs.hide()
	h_box_container_modifier.visible = has_modifier
	texture_rect_gamepad_button.texture = button_texture


func _show_keyboard(text: String) -> void:
	show()
	h_box_container_gamepad.hide()
	panel_container_inputs.show()
	label_action.show()
	texture_rect_action_icon.hide()
	label_action.text = text


func _show_keyboard_icon(icon: Texture2D) -> void:
	show()
	h_box_container_gamepad.hide()
	panel_container_inputs.show()
	texture_rect_action_icon.show()
	label_action.hide()
	texture_rect_action_icon.texture = icon


func _physics_process(_delta):
	if action_to_trigger == "ia_any":
		return

	var new_pressed = Input.is_action_pressed(action_to_trigger)
	if last_state_pressed != new_pressed:
		set_bg_color(BG_COLOR_PRESSED if new_pressed else BG_COLOR_NORMAL)
		panel_container_inputs.position = Vector2i(-1, -1) if new_pressed else Vector2i.ZERO
		label_text.text = text_up if new_pressed else text_down
		last_state_pressed = new_pressed


func mobile_on_panel_container_gui_input(event):
	if event is InputEventScreenTouch:
		if action_to_trigger.is_empty():
			return
		if event.pressed:
			Input.action_press(action_to_trigger)
		else:
			Input.action_release(action_to_trigger)
