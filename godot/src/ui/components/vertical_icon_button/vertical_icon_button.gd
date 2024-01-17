@tool
extends Button
@export var icon_unpressed:Texture2D
@export var icon_pressed:Texture2D
@export var label_value:String
@onready var texture_rect_icon = $VBoxContainer/MarginContainer/TextureRect_Icon
@onready var label = $VBoxContainer/HBoxContainer/Label



func _ready():
	_update_button()
	label.text = label_value
		
func _update_button():
	if button_pressed:
		texture_rect_icon.texture = icon_pressed
		self_modulate = Color("ffffff1e")
	else:
		texture_rect_icon.texture = icon_unpressed
		self_modulate = Color("ffffff00")


func _on_toggled(_toggled_on):
	_update_button() # Replace with function body.
