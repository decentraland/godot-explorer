@tool
class_name CategoryTag
extends PanelContainer

const CATEGORIES_ICONS_PATH = "res://assets/ui/places_categories/"

@onready var texture_rect: TextureRect = %TextureRect
@onready var label: Label = %Label


func set_category(category: String) -> void:
	if category == "poi":
		label.text = "point of interest"
	elif category == "gaming":
		label.text = "game"
	else:
		label.text = category
	texture_rect.texture = load(CATEGORIES_ICONS_PATH + category + ".svg")
