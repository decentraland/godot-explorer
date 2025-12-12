extends Button


@export var trigger_action = "ia_primary"


func _on_button_down() -> void:
	Input.action_press(trigger_action)


func _on_button_up() -> void:
	Input.action_release(trigger_action)
