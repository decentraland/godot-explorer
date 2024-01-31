@tool
extends TextureButton

@export var text_color = Color.WHITE:
	set(new_value):
		$Label.add_theme_color_override("font_color", new_value)
		text_color = new_value

@export var text_letter = "E":
	set(new_value):
		$Label.text = new_value
		text_letter = new_value

@export var trigger_action = "ia_primary"


func _on_button_down():
	Input.action_press(trigger_action)


func _on_button_up():
	Input.action_release(trigger_action)
