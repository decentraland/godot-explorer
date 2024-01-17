@tool
extends Button

@export var navigation:bool = false
@export var bg_color:Color = 'black'
@export var icon_color:Color = 'white'
@export var icon_source:Texture2D

@onready var texture_rect_icon = $MarginContainer/TextureRect_Icon

func _ready():
	if navigation:
		custom_minimum_size = Vector2(30,30)
	else:
		custom_minimum_size = Vector2(35,35)
		
	self_modulate = bg_color
	texture_rect_icon.self_modulate = icon_color
	texture_rect_icon.texture = icon_source




func _on_button_down():
	scale = Vector2(1.1,1.1)
	texture_rect_icon.self_modulate = Color(icon_color.r/2, icon_color.g/2, icon_color.b/2)

func _on_button_up():
	scale = Vector2(1,1)
	texture_rect_icon.self_modulate = icon_color
