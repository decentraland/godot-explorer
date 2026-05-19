extends Control

var collapsed: Vector2i = Vector2i(128, 0)
var expanded: Vector2i = Vector2i(128, 145)

@onready var panel = $Panel
@onready var control = $Panel/MarginContainer/Control
@onready var label_value = $Panel/MarginContainer/Control/VBoxContainer/HBoxContainer/Label_Value
@onready var button_more_or_less = $Panel/Button_MoreOrLess


# Called when the node enters the scene tree for the first time.
func _ready():
	panel.size = collapsed
	control.hide()


func set_parcel_scene_name(value: String):
	label_value.text = value


func _on_button_more_or_less_toggled(button_pressed):
	if button_pressed:
		panel.size = expanded
		control.show()
		button_more_or_less.text = "LESS"
	else:
		panel.size = collapsed
		control.hide()
		button_more_or_less.text = "MORE"
