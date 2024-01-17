extends ColorRect
const MY_PROFILE = preload("res://src/ui/components/profile/my_profile.tscn")
@onready var margin_container_content = $PanelContainer/MarginContainer_Content
@onready var icon_button_back = $PanelContainer/MarginContainer_Navigation/HBoxContainer/VBoxContainer_Back/IconButton_Back
@onready var icon_button_close = $PanelContainer/MarginContainer_Navigation/HBoxContainer/VBoxContainer_Close/IconButton_Close

func _ready():
	icon_button_close.hide()
	icon_button_back.hide()
	
func _on_gui_input(event):
	if event is InputEventScreenTouch:
		if event.pressed:
			close()

func close():
	hide()
	icon_button_close.hide()
	icon_button_back.hide()
	for child in margin_container_content.get_children():
		remove_child(child)
		
func openProfile():
	var profileNode = MY_PROFILE.instantiate()
	margin_container_content.add_child(profileNode)
	icon_button_close.show()
	show()


func _on_icon_button_close_pressed():
	close()
