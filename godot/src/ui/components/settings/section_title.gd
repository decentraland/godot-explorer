@tool
extends PanelContainer

@export var title: String = "Section":
	set(value):
		title = value
		if is_inside_tree() and label_title:
			label_title.text = title

@onready var label_title: Label = %Label_Title


func _ready():
	label_title.text = title


func set_font_size(size: int) -> void:
	if label_title.label_settings:
		label_title.label_settings.font_size = size
