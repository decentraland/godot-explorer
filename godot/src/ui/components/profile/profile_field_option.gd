class_name ProfileFieldOption
extends Control
signal change_editing(editing:bool)

@export var title:String = "title"
@export var icon:Texture

@onready var texture_rect_icon: TextureRect = %TextureRect_Icon
@onready var label_title: Label = %Label_Title
@onready var button_erase: Button = %Button_Erase
@onready var option_button: OptionButton = %OptionButton
@onready var label_value: Label = %Label_Value

var is_editing: bool = false

func _ready() -> void:
	_on_change_editing(false)
	label_title.text = title
	if icon:
		texture_rect_icon.texture = icon

func _on_option_button_item_selected(index: int) -> void:
	label_value.text = option_button.get_item_text(index)
	_update_erase_button_visibility()
				
func _on_change_editing(editing: bool) -> void:
	is_editing = editing
	if editing:
		self.show()
		option_button.show()
		label_value.hide()
	else:
		if option_button.selected <= 0:
			self.hide()
		option_button.hide()
		label_value.show()
	
	_update_erase_button_visibility()

func _update_erase_button_visibility() -> void:
	# Solo mostrar el botón de borrar si está en modo edición Y tiene un valor válido
	if is_editing and option_button.selected > 0:
		button_erase.show()
	else:
		button_erase.hide()

func _on_button_pressed() -> void:
	option_button.selected = 0
	_update_erase_button_visibility()

func add_option(option:String)-> void:
	option_button.add_item(option)

func select_option(index:int) -> void:
	option_button.selected = index
	_on_option_button_item_selected(index)
	_update_visibility()

func _update_visibility() -> void:
	# Si no está en modo edición, mostrar el campo solo si tiene un valor válido (índice > 0)
	if not option_button.visible:
		if option_button.selected > 0:
			self.show()
		else:
			self.hide()
