@tool
class_name CategoryTag
extends PanelContainer

@onready var label: Label = %Label


func set_category(category: String) -> void:
	label.text = category

	if category == "poi":
		label.text = tr("DISCOVER_POINT_OF_INTEREST")
