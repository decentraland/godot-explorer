extends Control

@onready
var label_action = $PanelContainer/MarginContainer/HBoxContainer/PanelContainer/MarginContainer/Label_Action
@onready var label_text = $PanelContainer/MarginContainer/HBoxContainer/Label_Text


func set_tooltip_data(text: String, action: String):
	var key: String
	var index: int = InputMap.get_actions().find(action.to_lower(), 0)
	if label_text:
		if index != -1:
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
			hide()
			printerr("Action doesn't exist ", action)
