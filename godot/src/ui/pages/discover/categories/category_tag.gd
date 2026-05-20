@tool
class_name CategoryTag
extends PanelContainer

const CATEGORIES_ICONS_PATH = "res://assets/ui/places_categories/"

@onready var texture_rect: TextureRect = %TextureRect
@onready var label: Label = %Label


func set_category(category: String) -> void:
	var icon_file_name = category
	label.text = category

	if category == "poi":
		label.text = "point of interest"
	elif category == "featured":
		icon_file_name = "poi"

	texture_rect.texture = load(CATEGORIES_ICONS_PATH + icon_file_name + ".svg")
