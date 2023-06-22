extends Panel

@onready var h_box_container = $HBoxContainer
@onready var label_action = $HBoxContainer/Label_Action
@onready var label_text = $HBoxContainer/Label_Text


func _open_with(action: String, text: String) -> void:
	label_action.text = action
	label_text.text = text
	self.show()
	self.size = h_box_container.size
