@tool
extends PanelContainer

## The title label is an OrientationLabel, so the per-orientation font size lives in the scene
## (portrait/landscape font size on Label_Title), not here.

@export var title: String = "Section":
	set(value):
		title = value
		if is_inside_tree() and label_title:
			label_title.text = title

@onready var label_title: Label = %Label_Title


func _ready():
	label_title.text = title
