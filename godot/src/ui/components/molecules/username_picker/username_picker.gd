class_name UsernamePicker
extends VBoxContainer

signal name_changed

var has_error: bool:
	get:
		return dcl_text_edit_username.error

var _is_claimed: bool = false
var _wallet_address: String = ""
var _minted_names: Array[String] = []
var _load_generation: int = 0
var _suppressing_text_signal: bool = false

@onready var dcl_text_edit_username: DclTextEdit = %DclTextEdit_Username
@onready var label_tag: Label = %Label_Tag
@onready var dropdown_list: DropdownList = %DropdownList
@onready var control_text_edit: Control = %Control_TextEdit
@onready var button_unique: Button = %Button_Unique
@onready var button_non_unique: Button = %Button_NonUnique
@onready var v_box_container_tabs: VBoxContainer = %VBoxContainer_Tabs


func _ready() -> void:
	var button_group = ButtonGroup.new()
	button_group.allow_unpress = false
	button_unique.toggle_mode = true
	button_non_unique.toggle_mode = true
	button_unique.button_group = button_group
	button_non_unique.button_group = button_group

	dropdown_list.are_names = true
	dropdown_list.without_placeholder = true
	dcl_text_edit_username.dcl_text_edit_changed.connect(_on_text_changed)
	dropdown_list.item_selected.connect(_on_name_selected)

	dropdown_list.hide()
	control_text_edit.hide()
	v_box_container_tabs.hide()


func populate(current_name: String, wallet_address: String, is_claimed: bool) -> void:
	_wallet_address = wallet_address
	_is_claimed = is_claimed
	_minted_names.clear()

	dcl_text_edit_username.set_text_value(current_name)
	_update_tag_visibility()

	dropdown_list.clear()
	dropdown_list.hide()

	if is_claimed:
		# We know names exist — show tabs immediately to avoid flash
		v_box_container_tabs.show()
		button_unique.set_pressed_no_signal(true)
		button_non_unique.set_pressed_no_signal(false)
		control_text_edit.hide()
	else:
		# Unknown if names exist yet — default to text edit while loading
		v_box_container_tabs.hide()
		button_non_unique.set_pressed_no_signal(true)
		button_unique.set_pressed_no_signal(false)
		control_text_edit.show()

	_load_generation += 1
	_async_load_names(_load_generation)


func get_name_value() -> String:
	if button_unique.button_pressed and not _minted_names.is_empty():
		var idx = dropdown_list.selected
		if idx >= 0 and idx < _minted_names.size():
			return _minted_names[idx]
	return dcl_text_edit_username.get_text_value()


func get_is_claimed() -> bool:
	if button_unique.button_pressed:
		return dropdown_list.selected >= 0 and dropdown_list.selected < _minted_names.size()
	var current_name = dcl_text_edit_username.get_text_value()
	for minted_name in _minted_names:
		if minted_name == current_name:
			return true
	return false


func _update_tag_visibility() -> void:
	if get_is_claimed() or _wallet_address.length() < 4:
		label_tag.hide()
	else:
		label_tag.text = "#" + _wallet_address.substr(_wallet_address.length() - 4, 4)
		label_tag.show()


func _async_load_names(generation: int) -> void:
	var response = await NamesRequest.async_request_all_names()
	if not is_instance_valid(self) or generation != _load_generation:
		return
	if response == null or response.elements.is_empty():
		# No minted names: hide tabs, show text edit with current name
		v_box_container_tabs.hide()
		dropdown_list.hide()
		control_text_edit.show()
		return

	_minted_names.clear()
	dropdown_list.clear()

	var current_name = dcl_text_edit_username.get_text_value()
	var preselect_index = 0

	for i in range(response.elements.size()):
		var element_name: String = response.elements[i].name
		_minted_names.append(element_name)
		dropdown_list.add_item(element_name)
		if _is_claimed and element_name == current_name:
			preselect_index = i

	dropdown_list.select(preselect_index)

	# Names exist: ensure tabs are visible
	v_box_container_tabs.show()
	if button_unique.button_pressed:
		dropdown_list.show()


func _on_text_changed() -> void:
	if _suppressing_text_signal:
		return
	_update_tag_visibility()
	name_changed.emit()


func _on_name_selected(_index: int) -> void:
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
	_update_tag_visibility()
	name_changed.emit()


func _on_button_non_unique_toggled(toggled_on: bool) -> void:
	_hide_all()
	if not toggled_on:
		return
	_suppressing_text_signal = true
	dcl_text_edit_username.set_text_value("")
	_suppressing_text_signal = false
	control_text_edit.show()
	_update_tag_visibility()
	name_changed.emit()
