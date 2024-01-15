@tool
extends Button

@export var navigation:bool = false
@export var bg_color:Color = 'black'
@export var icon_color:Color = 'white'
@export var icon_source:Texture2D

@onready var texture_rect_icon = $MarginContainer/TextureRect_Icon

func _ready():
	if navigation:
		size = Vector2(26,26)
	else:
		size = Vector2(30,30)
		
	self_modulate = bg_color
	texture_rect_icon.self_modulate = icon_color
	texture_rect_icon.texture = icon_source


