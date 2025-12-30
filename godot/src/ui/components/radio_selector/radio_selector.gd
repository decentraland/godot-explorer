@tool
class_name RadioSelector
extends Control

signal select_item(index: int, item: String)

@export var items: Array[String]:
	set(new_value):
		items = new_value
		_refresh_list()

@export var selected: int = 0:
	set(new_value):
		selected = new_value
		if selected < 0 or selected >= get_child_count():
			return
		var radio_button = get_child(selected)
		if is_instance_valid(radio_button) and radio_button is CheckBox:
			for child in self.get_children():
				child.set_pressed_no_signal(false)

			radio_button.set_pressed_no_signal(true)

var button_group = ButtonGroup.new()


func add_item(item: String):
	items.push_back(item)


func clear():
	items = []
	_refresh_list()


func select_by_item(p_item: String):
	var i: int = 0
	for item in items:
		if item == p_item:
			selected = i
			break
		i += 1


func _add_item(item: String):
	var index = get_children().size()
	var radio_button = CheckBox.new()
	radio_button.button_group = button_group
	radio_button.text = item
	radio_button.pressed.connect(_on_select_item.bind(index, item))
	add_child(radio_button)


func _refresh_list():
	for child in self.get_children():
		remove_child(child)
		child.queue_free()

	for item in items:
		_add_item(item)

	selected = selected


func _on_select_item(index: int, item: String):
	selected = index
	select_item.emit(index, item)
