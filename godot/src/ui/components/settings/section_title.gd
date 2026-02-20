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
