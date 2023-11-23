extends Button

signal select_color

@export var background_color: Color:
	set(value):
		background_color = value
		if is_instance_valid(panel_color):
			panel_color.modulate = background_color

@onready var panel_container_border = $PanelContainer_Border
@onready var panel_color = $Panel_Color


func _ready():
	panel_container_border.hide()
	panel_color.modulate = background_color


func _on_toggled(is_button_pressed):
	if is_button_pressed:
		emit_signal("select_color", background_color)
		panel_container_border.show()
	else:
		panel_container_border.hide()
