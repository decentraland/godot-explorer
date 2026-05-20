extends Control

@onready var texture_rect_item = $TextureRect_Item
@onready var animation_player = $AnimationPlayer


func _ready():
	pass


func select():
	animation_player.play("select")


func unselect():
	animation_player.play("unselect")
