extends Control

var action_to_trigger: String = ""

@onready
var label_action = $PanelContainer/MarginContainer/HBoxContainer/PanelContainer/MarginContainer/Label_Action
@onready var label_text = $PanelContainer/MarginContainer/HBoxContainer/Label_Text


func _ready():
	if Global.is_mobile:
		self.gui_input.connect(self.mobile_on_panel_container_gui_input)


func set_tooltip_data(text: String, action: String):
	var key: String
	var action_lower: String = action.to_lower()
	var index: int = InputMap.get_actions().find(action_lower, 0)
	if label_text:
		if index == -1 and action_lower == "ia_any":
			key = "Any"
		elif index != -1:
			var event = InputMap.action_get_events(InputMap.get_actions()[index])[0]
			if event is InputEventKey:
				key = char(event.unicode).to_upper()
			elif event is InputEventMouseButton:
				if event.button_index == 1:
					key = "Mouse Left Button"
				if event.button_index == 2:
					key = "Mouse Right Button"
				if event.button_index == 0:
					key = "Mouse Wheel Button"

		if not key.is_empty():
			show()
			action_to_trigger = action_lower
			label_action.text = key
			label_text.text = text
		else:
			hide()
			action_to_trigger = ""
			printerr("Action doesn't exist ", action)


func mobile_on_panel_container_gui_input(event):
	if action_to_trigger.is_empty():
		return

	if event is InputEventMouseButton:
		if event.pressed:
			Input.action_press(action_to_trigger)
		else:
			Input.action_release(action_to_trigger)
