@tool
class_name AboutData
extends VBoxContainer

@export var title: String = "":
	set(value):
		title = value
		if is_node_ready():
			%Label_Title.text = value

@export var icon: Texture2D:
	set(value):
		icon = value
		if is_node_ready():
			%TextureRect_Icon.texture = value


func _ready():
	%Label_Title.text = title
	%TextureRect_Icon.texture = icon


func set_value(text: String) -> void:
	visible = not text.is_empty()
	%Label_Value.text = text


func has_value() -> bool:
	return not %Label_Value.text.is_empty()
