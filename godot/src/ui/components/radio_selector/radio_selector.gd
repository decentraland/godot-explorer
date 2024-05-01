@tool
extends HBoxContainer

signal select_item(index: int, item: String)

var button_group = ButtonGroup.new()

@export var items: Array[String]:
	set(new_value):
		items = new_value
		for child in self.get_children():
			remove_child(child)
		
		for item in new_value:
			add_item(item)
		
		selected = selected

@export var selected: int = 0:
	set(new_value):
		selected = new_value
		var radio_button: CheckBox = get_child(selected) as CheckBox
		if is_instance_valid(radio_button):
			for child in self.get_children():
				child.set_pressed_no_signal(false)

			radio_button.set_pressed_no_signal(true)

func clear():
	items = []

func add_item(item: String):
	var index = get_children().size()
	var radio_button = CheckBox.new()
	radio_button.button_group = button_group
	radio_button.text = item
	radio_button.pressed.connect(_on_select_item.bind(index, item))
	add_child(radio_button)

func _on_select_item(index: int, item: String):
	selected = index
	select_item.emit(index, item)
