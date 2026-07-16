class_name UsernamePicker
extends VBoxContainer

signal name_changed

const DROPDOWN_PLACEHOLDER_INDEX = 0

var _is_claimed: bool = false
var _wallet_address: String = ""
var _minted_names: Array[String] = []
var _updating_from_dropdown: bool = false

var has_error: bool:
	get:
		return dcl_text_edit_username.error

@onready var dcl_text_edit_username: DclTextEdit = %DclTextEdit_Username
@onready var label_tag: Label = %Label_Tag
@onready var dropdown_list: DropdownList = %DropdownList
@onready var control_text_edit: Control = %Control_TextEdit
@onready var button_unique: Button = %Button_Unique
@onready var button_non_unique: Button = %Button_NonUnique


func _ready() -> void:
	var button_group = ButtonGroup.new()
	button_group.allow_unpress = false
	button_unique.toggle_mode = true
	button_non_unique.toggle_mode = true
	button_unique.button_group = button_group
	button_non_unique.button_group = button_group

	dropdown_list.are_names = true
	dcl_text_edit_username.dcl_text_edit_changed.connect(_on_text_changed)
	dropdown_list.item_selected.connect(_on_name_selected)

	dropdown_list.hide()
	control_text_edit.hide()


func populate(current_name: String, wallet_address: String, is_claimed: bool) -> void:
	_wallet_address = wallet_address
	_is_claimed = is_claimed
	_minted_names.clear()

	dcl_text_edit_username.set_text_value(current_name)
	_update_tag_visibility()

	dropdown_list.clear()
	dropdown_list.hide()

	if is_claimed:
		button_unique.set_pressed_no_signal(true)
		button_non_unique.set_pressed_no_signal(false)
		control_text_edit.hide()
	else:
		button_non_unique.set_pressed_no_signal(true)
		button_unique.set_pressed_no_signal(false)
		control_text_edit.show()

	_async_load_names()


func get_name_value() -> String:
	return dcl_text_edit_username.get_text_value()


func get_is_claimed() -> bool:
	return _is_claimed


func _update_tag_visibility() -> void:
	if _is_claimed or _wallet_address.length() < 4:
		label_tag.hide()
	else:
		label_tag.text = "#" + _wallet_address.substr(_wallet_address.length() - 4, 4)
		label_tag.show()


func _async_load_names() -> void:
	var response = await NamesRequest.async_request_all_names()
	if not is_instance_valid(self):
		return
	if response == null or response.elements.is_empty():
		return

	_minted_names.clear()
	dropdown_list.clear()
	dropdown_list.add_item("Select a name", 0)
	dropdown_list.placeholder_index = DROPDOWN_PLACEHOLDER_INDEX
	dropdown_list.select(DROPDOWN_PLACEHOLDER_INDEX)

	var current_name_lower = dcl_text_edit_username.get_text_value().to_lower()
	var preselect_index = 0

	for i in range(response.elements.size()):
		var element_name: String = response.elements[i].name
		_minted_names.append(element_name)
		dropdown_list.add_item(element_name, i + 1)
		if _is_claimed and element_name.to_lower() == current_name_lower:
			preselect_index = i + 1

	if preselect_index > 0:
		dropdown_list.select(preselect_index)

	if button_unique.button_pressed:
		dropdown_list.show()


func _on_text_changed() -> void:
	if _updating_from_dropdown:
		return
	var typed = dcl_text_edit_username.get_text_value().to_lower()
	_is_claimed = false
	for minted_name in _minted_names:
		if minted_name.to_lower() == typed:
			_is_claimed = true
			break
	_update_tag_visibility()
	name_changed.emit()


func _on_name_selected(index: int) -> void:
	var name_index = index - 1
	if name_index < 0 or name_index >= _minted_names.size():
		return
	var selected_name: String = _minted_names[name_index]
	_updating_from_dropdown = true
	dcl_text_edit_username.set_text_value(selected_name)
	_updating_from_dropdown = false
	_is_claimed = true
	_update_tag_visibility()
	name_changed.emit()


func _hide_all() -> void:
	dropdown_list.hide()
	control_text_edit.hide()


func _on_button_unique_toggled(toggled_on: bool) -> void:
	_hide_all()
	if not toggled_on:
		return
	if not _minted_names.is_empty():
		dropdown_list.show()
	var name_index = dropdown_list.selected - 1
	if name_index >= 0 and name_index < _minted_names.size():
		_updating_from_dropdown = true
		dcl_text_edit_username.set_text_value(_minted_names[name_index])
		_updating_from_dropdown = false
		_is_claimed = true
		_update_tag_visibility()
	name_changed.emit()


func _on_button_non_unique_toggled(toggled_on: bool) -> void:
	_hide_all()
	if not toggled_on:
		return
	dcl_text_edit_username.set_text_value("")
	control_text_edit.show()
	_is_claimed = false
	_update_tag_visibility()
	name_changed.emit()
