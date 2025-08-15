extends ColorRect

@onready var label_url: Label = %Label_Url


func _on_gui_input(event: InputEvent) -> void:
	if event is InputEventScreenTouch:
		if event.pressed:
			close()


func close() -> void:
	label_url.text = ""
	hide()


func open(url: String) -> void:
	show()
	label_url.text = url


func _on_button_go_to_link_continue_pressed() -> void:
	if label_url.text.length() != 0:
		Global.open_url(label_url.text)
		close()


func _on_button_go_to_link_cancel_pressed() -> void:
	close()
