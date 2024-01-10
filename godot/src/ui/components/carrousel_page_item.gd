extends Control
@onready var texture_rect_item = $TextureRect_Item


# Called when the node enters the scene tree for the first time.
func _ready():
	pass # Replace with function body.


# Called every frame. 'delta' is the elapsed time since the previous frame.
func _process(delta):
	pass

func select():
	texture_rect_item.self_modulate = '#f52758'
	
func unselect():
	texture_rect_item.self_modulate = '#ffffff'
