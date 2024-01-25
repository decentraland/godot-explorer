extends PanelContainer

var action_to_trigger: String = ""

@onready var label_action = %Label_Action
@onready var texture_rect_action_icon = %TextureRect_ActionIcon

@onready var label_text = %Label_Text

@onready var icon_left_click = preload("res://assets/themes/icons/LeftClickIcn.png")
@onready var icon_interactive_pointer = preload("res://assets/themes/icons/InteractiveIcon.svg")

@onready var panel_action = $MarginContainer/HBoxContainer/PanelContainer

var theme_bg

var text_down := ""
var text_up := ""
var last_state_pressed := false

const BG_COLOR_NORMAL: String = "#00000080"
const BG_COLOR_PRESSED: String = "#44444480"

var stylebox: StyleBox

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

	var key: Variant = null
	var action_lower: String = action.to_lower()
	var index: int = InputMap.get_actions().find(action_lower, 0)
	if label_text:
		if index == -1 and action_lower == "ia_any":
			key = "Any"
		elif index != -1:
			var event = InputMap.action_get_events(InputMap.get_actions()[index])[0]
			if event is InputEventKey:
				key = icon_interactive_pointer if Global.is_mobile() else char(event.unicode).to_upper()
			elif event is InputEventMouseButton:
				if event.button_index == 1:
					key = icon_left_click

		if key != null:
			show()
			if key is String:
				set_action_text(key)
			else:
				set_action_icon(key)

			action_to_trigger = action_lower
			label_text.text = text_down
		else:
			hide()
			action_to_trigger = ""
			printerr("Action doesn't exist ", action)

func set_action_icon(icon):
	texture_rect_action_icon.show()
	label_action.hide()
	texture_rect_action_icon.texture = icon
	
func set_action_text(text: String):
	label_action.show()
	texture_rect_action_icon.hide()
	label_action.text = text


func _physics_process(delta):
	var new_pressed = Input.is_action_pressed(action_to_trigger)
	if last_state_pressed != new_pressed:
		set_bg_color(BG_COLOR_PRESSED if new_pressed else BG_COLOR_NORMAL)
		panel_action.position = Vector2i(-1, -1) if new_pressed else Vector2i.ZERO
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
