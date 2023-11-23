extends Control

signal toggle_color_panel(toggled, color_target)

enum ColorTarget { SKIN, HAIR, EYE }

@export var color_target := ColorTarget.SKIN

@onready var button = $Button
@onready var right_arrow = $Button/Right_Arrow

@onready var color_rect = $Button/ColorRect


func _on_button_toggled(button_pressed):
	set_toggled(button_pressed)
	emit_signal("toggle_color_panel", button_pressed, color_target)


func set_color(color: Color) -> void:
	color_rect.color = color


func set_text(text: String) -> void:
	button.text = text + "           "


func set_toggled(value):
	if value:
		right_arrow.rotation_degrees = -90
	else:
		right_arrow.rotation_degrees = 90
