@tool
extends PanelContainer

const ARROW_DOWN = preload("uid://cjtlaexth4ts0")
const ARROW_UP = preload("uid://b8b8s3aer40p4")

@export var title: String = "":
	set(value):
		title = value
		if is_node_ready():
			label_title.text = value

@export var description: String = "":
	set(value):
		description = value
		if is_node_ready():
			label_description.text = value

@onready var label_title: Label = %Label_Title
@onready var label_description: Label = %Label_Description
@onready var texture_rect: TextureRect = %TextureRect
@onready var button: Button = %Button


func _ready() -> void:
	label_title.text = title
	label_description.text = description
	label_description.hide()
	_update_arrow_icon(false)


func _on_button_toggled(toggled_on: bool) -> void:
	if Engine.is_editor_hint():
		return
	label_description.visible = toggled_on
	texture_rect.texture = ARROW_UP if toggled_on else ARROW_DOWN


func _update_arrow_icon(is_open: bool) -> void:
	texture_rect.texture = ARROW_UP if is_open else ARROW_DOWN
