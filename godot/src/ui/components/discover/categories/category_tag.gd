@tool
class_name CategoryTag
extends PanelContainer

const categories_icons_path = "res://assets/ui/places_categories/"

@onready var texture_rect: TextureRect = %TextureRect
@onready var label: Label = %Label

func set_category(category:String) -> void:
	if category == "poi":
		label.text = "point of interest"
	else:
		label.text = category
	texture_rect.texture = load(categories_icons_path + category + ".svg")
	
