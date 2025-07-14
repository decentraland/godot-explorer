extends Control
signal change_editing(editing:bool)

@export var title:String = "title"
@export var icon:Texture

@onready var texture_rect_icon: TextureRect = %TextureRect_Icon
@onready var label_title: Label = %Label_Title
@onready var button_erase: Button = %Button_Erase
@onready var label_value: Label = %Label_Value
@onready var text_edit_value: TextEdit = %TextEdit_Value


func _ready() -> void:
	_on_change_editing(false)
	label_title.text = title
	if icon:
		texture_rect_icon.texture = icon


func _on_change_editing(editing: bool) -> void:
	if editing:
		self.show()
		text_edit_value.show()
		if text_edit_value.text != "":
			button_erase.show()
		label_value.hide()
	else:
		if text_edit_value.text == "":
			self.hide()
		button_erase.hide()
		text_edit_value.hide()
		label_value.show()


func _on_button_pressed() -> void:
	text_edit_value.text = ""
	button_erase.hide()


func _on_text_edit_value_text_changed() -> void:
	label_value.text = text_edit_value.text
	if text_edit_value.text != "":
		button_erase.show()
	else:
		button_erase.hide()
