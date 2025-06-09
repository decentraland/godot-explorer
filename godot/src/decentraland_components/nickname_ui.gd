@tool
extends MarginContainer

@export var mic_enabled := false:
	set(value):
		mic_enabled = value
		mic_enabled_icon.visible = mic_enabled

@export var nickname := "nickname":
	set(value):
		nickname = value
		nickname_label.text = nickname

@export var tag := "xxxx":
	set(value):
		tag = value
		tag_label.text = tag

@export var nickname_color := Color(1, 1, 1):  # Default to white
	set(value):
		nickname_color = value
		nickname_label.add_theme_color_override("font_color", nickname_color)

@onready var mic_enabled_icon = %MicEnabled
@onready var tag_label = %Tag
@onready var nickname_label = %Nickname
