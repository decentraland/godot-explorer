extends ColorRect
const MY_PROFILE = preload("res://src/ui/components/profile/my_profile.tscn")
@onready var margin_container_content = $PanelContainer/MarginContainer_Content
@onready var icon_button_back = $PanelContainer/MarginContainer_Navigation/HBoxContainer/VBoxContainer_Back/IconButton_Back
@onready var icon_button_close = $PanelContainer/MarginContainer_Navigation/HBoxContainer/VBoxContainer_Close/IconButton_Close

func _ready():
	hide()
	modulate = Color(1, 1, 1, 0)
	icon_button_close.hide()
	icon_button_back.hide()
	#open(MY_PROFILE, false)
	
func _on_gui_input(event):
	if event is InputEventScreenTouch:
		if event.pressed:
			close()

func close():
	icon_button_close.hide()
	icon_button_back.hide()
	for child in margin_container_content.get_children():
		remove_child(child)
	var tween_m = create_tween()
	tween_m.tween_property(self, "modulate", Color(1, 1, 1, 0), 0.3)
	var tween_h = create_tween()
	tween_h.tween_callback(self.hide).set_delay(0.3)

		
func open(childNode, closeButton:bool):
	
	var content = childNode.instantiate()
	#margin_container_content.size = Vector2(content.get_size().x, size.y)
	margin_container_content.add_child(content)
	if closeButton:
		icon_button_close.show()
	show()
	var tween_m = create_tween()
	tween_m.tween_property(self, "modulate", Color(1, 1, 1), 0.3)
	

func _on_icon_button_close_pressed():
	close()
