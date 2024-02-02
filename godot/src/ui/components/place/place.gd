extends VBoxContainer

const DISLIKE_SOLID = preload("res://assets/ui/Icn/Dislike solid.svg")
const DISLIKE = preload("res://assets/ui/Icn/Dislike.svg")
const LIKE_SOLID = preload("res://assets/ui/Icn/Like solid.svg")
const LIKE = preload("res://assets/ui/Icn/Like.svg")
const HOME = preload("res://assets/ui/Icn/Home.svg")
const HOME_OUTLINE = preload("res://assets/ui/Icn/HomeOutline.svg")
const STAR_OUTLINE = preload("res://assets/ui/Icn/StarOutline.svg")
const STAR_SOLID = preload("res://assets/ui/Icn/StarSolid.svg")

@onready
var button_like = $MarginContainer/VBoxContainer/MarginContainer/VBoxContainer/HBoxContainer2/Button_Like
@onready
var button_dislike = $MarginContainer/VBoxContainer/MarginContainer/VBoxContainer/HBoxContainer2/Button_Dislike
@onready
var button_fav = $MarginContainer/VBoxContainer/MarginContainer/VBoxContainer/HBoxContainer2/Button_Fav
@onready
var button_share = $MarginContainer/VBoxContainer/MarginContainer/VBoxContainer/HBoxContainer2/Button_Share
@onready
var button_home = $MarginContainer/VBoxContainer/MarginContainer/VBoxContainer/HBoxContainer2/Button_Home


func _on_button_like_toggled(toggled_on):
	if toggled_on:
		button_like.icon = LIKE_SOLID
	else:
		button_like.icon = LIKE


func _on_button_dislike_toggled(toggled_on):
	if toggled_on:
		button_dislike.icon = DISLIKE_SOLID
	else:
		button_dislike.icon = DISLIKE


func _on_button_fav_toggled(toggled_on):
	if toggled_on:
		button_fav.icon = STAR_SOLID
	else:
		button_fav.icon = STAR_OUTLINE


func _on_button_home_toggled(toggled_on):
	if toggled_on:
		button_home.icon = HOME
	else:
		button_home.icon = HOME_OUTLINE
