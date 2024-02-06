extends Control
@onready var texture_rect_background = $MarginContainer/DiscoverCarrouselItem/TextureRect_Background
@onready
var label_title = $MarginContainer/DiscoverCarrouselItem/Panel/MarginContainer/VBoxContainer/Label_Title
@onready
var label_description = $MarginContainer/DiscoverCarrouselItem/Panel/MarginContainer/VBoxContainer/Label_Description
@onready
var label_online = $MarginContainer/DiscoverCarrouselItem/TextureRect_Background/MarginContainer/VBoxContainer/HBoxContainer/Panel/MarginContainer/HBoxContainer/Label_Online
@onready
var label_views = $MarginContainer/DiscoverCarrouselItem/TextureRect_Background/MarginContainer/VBoxContainer/VBoxContainer/HBoxContainer2/HBoxContainer/Label_Views
@onready
var label_liked = $MarginContainer/DiscoverCarrouselItem/TextureRect_Background/MarginContainer/VBoxContainer/VBoxContainer/HBoxContainer2/HBoxContainer2/Label_Liked
@onready var animation_player = $AnimationPlayer


func _ready():
	set_views(1500)


func set_title(title: String):
	label_title.text = title


func set_description(description: String):
	label_description.text = description


func set_views(views: int):
	label_views.text = _format_number(views)


func set_online(online: int):
	label_online.text = str(online)


func _format_number(num: int):
	if num < 1e3:
		return num
	if num < 1e6:
		return str(ceil(num / 1000.0)) + "k"
	return str(floor(num / 1000000.0)) + "M"


func _on_gui_input(event):
	pass # Replace with function body.
