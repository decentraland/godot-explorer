class_name ProfileFieldText
extends Control
signal change_editing(editing:bool)

@export var title:String = "Title"
@export var instructions:String = "Here you can explain how use this field"
@export var icon:Texture
@export var only_field:bool = false

@onready var texture_rect_icon: TextureRect = %TextureRect_Icon
@onready var label_title: Label = %Label_Title
@onready var button_erase: Button = %Button_Erase
@onready var label_value: Label = %Label_Value
@onready var text_edit_value: TextEdit = %TextEdit_Value
@onready var h_box_container: HBoxContainer = %HBoxContainer

var editing: bool = false

func _ready() -> void:
	if only_field:
		h_box_container.hide()
	_on_change_editing(false)
	label_title.text = title
	if icon:
		texture_rect_icon.texture = icon


func _on_change_editing(status: bool) -> void:
	editing = status
	if editing:
		if label_value.text != "":
			text_edit_value.text = label_value.text
	_update_visibility()


func _on_text_edit_value_text_changed() -> void:
	set_text(text_edit_value.text)


func set_text(value:String, field_too: bool = false) -> void:
	label_value.text = value
	if field_too:
		text_edit_value.text = value
	_update_visibility()


func _update_visibility() -> void:
	if editing:
		show()
		text_edit_value.show()
		label_value.hide()
		if label_value.text != "":
			button_erase.show()
		else:
			button_erase.hide()
	else:
		text_edit_value.hide()
		button_erase.hide()
		label_value.show()
		if label_value.text != "":
			show()
		else:
			hide()


func _on_button_erase_pressed() -> void:
	set_text("", true)
	button_erase.hide()
