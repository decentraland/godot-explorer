extends Control

@onready
var label_action = $PanelContainer/MarginContainer/HBoxContainer/PanelContainer/MarginContainer/Label_Action
@onready var label_text = $PanelContainer/MarginContainer/HBoxContainer/Label_Text

var action_to_trigger: String = ""

func _ready():
	if Global.is_mobile:
		self.gui_input.connect(self.mobile_on_panel_container_gui_input)
		
func set_tooltip_data(text: String, action: String):
	var key: String
	var index: int = InputMap.get_actions().find(action.to_lower(), 0)
	if label_text:
		if index != -1:
			action_to_trigger = action.to_lower()
			show()
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
			label_action.text = key
			label_text.text = text
		else:
			action_to_trigger = ""
			hide()
			printerr("Action doesn't exist ", action)


func mobile_on_panel_container_gui_input(event):
	if event is InputEventMouseButton:
		if event.pressed:
			Input.action_press(action_to_trigger)
		else:
			Input.action_release(action_to_trigger)
