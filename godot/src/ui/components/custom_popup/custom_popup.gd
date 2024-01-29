extends ColorRect

@onready var margin_container_content = $PanelContainer/MarginContainer_Content
@onready var button_back = $PanelContainer/MarginContainer_Navigation/HBoxContainer/Button_Back
@onready var button_close = $PanelContainer/MarginContainer_Navigation/HBoxContainer/Button_Close


func _ready():
	hide()
	modulate = Color(1, 1, 1, 0)
	button_close.hide()
	button_back.hide()


func _on_gui_input(event):
	if event is InputEventScreenTouch:
		if event.pressed:
			close()


func close():
	button_close.hide()
	button_back.hide()
	for child in margin_container_content.get_children():
		remove_child(child)
	var tween_m = create_tween()
	tween_m.tween_property(self, "modulate", Color(1, 1, 1, 0), 0.3)
	var tween_h = create_tween()
	tween_h.tween_callback(self.hide).set_delay(0.3)


func open(child_node, close_button: bool):
	var content = child_node.instantiate()
	#margin_container_content.size = Vector2(content.get_size().x, size.y)
	margin_container_content.add_child(content)
	if close_button:
		button_close.show()
	show()
	var tween_m = create_tween()
	tween_m.tween_property(self, "modulate", Color(1, 1, 1), 0.3)


func _on_button_close_pressed():
	close()
