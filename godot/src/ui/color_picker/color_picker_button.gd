extends Button

signal toggle_color_panel(toggled, color_target)

enum ColorTarget { SKIN, HAIR, EYE }

@export var color_target := ColorTarget.SKIN

@onready var button = $Button
@onready var color_rect = $ColorRect


func set_color(color: Color) -> void:
	color_rect.color = color


func _on_button_color_picker_toggled(toggled_on):
	emit_signal("toggle_color_panel", button_pressed, color_target)
