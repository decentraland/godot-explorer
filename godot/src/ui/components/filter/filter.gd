extends VBoxContainer

const CHECK = preload("res://assets/ui/Check.svg")
const DOWN_ARROW = preload("res://assets/ui/DownArrow.svg")
const UP_ARROW = preload("res://assets/ui/UpArrow.svg")

@onready
var button_collection = $Content/MarginContainer/ScrollContainer/VBoxContainer/MarginContainer2/VBoxContainer_Collection/Button_Collection
@onready
var button_sort_by = $Content/MarginContainer/ScrollContainer/VBoxContainer/MarginContainer3/VBoxContainer_Sort/Button_SortBy
@onready
var v_box_container_collection = $Content/MarginContainer/ScrollContainer/VBoxContainer/MarginContainer2/VBoxContainer_Collection
@onready
var v_box_container_sort = $Content/MarginContainer/ScrollContainer/VBoxContainer/MarginContainer3/VBoxContainer_Sort


func _ready():
	_update_icons()
	button_collection.button_pressed = false
	button_sort_by.button_pressed = false
	for child in v_box_container_collection.get_children():
		if child.get_index() > 0:
			child.hide()

	for child in v_box_container_collection.get_children():
		if child.get_index() > 0:
			child.hide()


func _on_button_sort_by_toggled(toggled_on):
	if toggled_on:
		button_sort_by.icon = UP_ARROW
	else:
		button_sort_by.icon = DOWN_ARROW
	for child in v_box_container_sort.get_children():
		if child.get_index() > 0:
			if toggled_on:
				child.show()
			else:
				child.hide()


func _on_button_collection_toggled(toggled_on):
	if toggled_on:
		button_collection.icon = UP_ARROW
	else:
		button_collection.icon = DOWN_ARROW
	for child in v_box_container_collection.get_children():
		if child.get_index() > 0:
			if toggled_on:
				child.show()
			else:
				child.hide()


func _update_icons():
	for child in v_box_container_collection.get_children():
		if child is Button and child.get_index() > 0:
			if child.button_pressed:
				child.icon = CHECK
			else:
				child.icon = null
	for child in v_box_container_sort.get_children():
		if child is Button and child.get_index() > 0:
			if child.button_pressed:
				child.icon = CHECK
			else:
				child.icon = null


func _on_button_c_2_pressed():
	_update_icons()


func _on_button_c_1_pressed():
	_update_icons()


func _on_button_newest_pressed():
	_update_icons()


func _on_button_oldest_pressed():
	_update_icons()


func _on_button_rarest_pressed():
	_update_icons()


func _on_button_less_rare_pressed():
	_update_icons()


func _on_button_pressed():
	_close()


func _close():
	var tween = get_tree().create_tween()
	tween.tween_property(self, "modulate", Color.TRANSPARENT, 0.3)
	await tween.finished
	hide()
