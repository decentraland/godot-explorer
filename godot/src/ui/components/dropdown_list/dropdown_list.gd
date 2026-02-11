@tool
class_name DropdownList
extends Control

signal item_selected(index: int)

## Title displayed above the dropdown button. Hidden when empty.
@export var title: String = "":
	set(value):
		title = value
		if is_node_ready():
			_update_title()

## Description displayed below the dropdown button. Hidden when empty.
@export var description: String = "":
	set(value):
		description = value
		if is_node_ready():
			_update_description()

## Maximum height of the popup panel before it starts scrolling.
@export var max_popup_height: float = 200.0

const DROPDOWN_ITEM_SCENE = preload("res://src/ui/components/dropdown_list/dropdown_item.tscn")

@onready var _vbox: VBoxContainer = $VBoxContainer
@onready var _title_label: Label = %Label_Title
@onready var _description_label: Label = %Label_Description
@onready var _selected_label: Label = %Label_Selected
@onready var _arrow_icon: TextureRect = %TextureRect_Arrow
@onready var _button_panel: PanelContainer = %PanelContainer_Button
@onready var _popup_layer: Control = %PopupLayer
@onready var _popup_panel: PanelContainer = %PanelContainer_Popup
@onready var _scroll_container: ScrollContainer = %ScrollContainer
@onready var _items_container: VBoxContainer = %VBoxContainer_Items

var _items: Array[Dictionary] = []
var selected: int = -1
var _is_open: bool = false

var _style_normal: StyleBoxFlat = load("res://assets/themes/dropdown_normal.tres")
var _style_hover: StyleBoxFlat = load("res://assets/themes/dropdown_normal.tres")
var _style_pressed: StyleBoxFlat = load("res://assets/themes/dropdown_selected.tres")



func _ready():
	_update_title()
	_update_description()
	add_item("ASD1", 0)
	add_item("ASD2", 1)
	add_item("ASD3", 2)
	
	if Engine.is_editor_hint():
		return

	_update_selected_text()

	_button_panel.gui_input.connect(_on_button_gui_input)
	_button_panel.mouse_entered.connect(_on_button_mouse_entered)
	_button_panel.mouse_exited.connect(_on_button_mouse_exited)

	_popup_layer.gui_input.connect(_on_popup_layer_gui_input)


# -- OptionButton-compatible API ---------------------------------------------


func add_item(label: String, id: int = -1) -> void:
	var actual_id := id if id >= 0 else _items.size()
	_items.append({text = label, id = actual_id})


func clear() -> void:
	_items.clear()
	selected = -1
	_update_selected_text()
	if _is_open:
		_close_popup()


func remove_item(idx: int) -> void:
	if idx < 0 or idx >= _items.size():
		return
	_items.remove_at(idx)
	if selected == idx:
		selected = -1
		_update_selected_text()
	elif selected > idx:
		selected -= 1


func get_item_count() -> int:
	return _items.size()


func get_item_text(idx: int) -> String:
	if idx >= 0 and idx < _items.size():
		return _items[idx].text
	return ""


func get_item_id(idx: int) -> int:
	if idx >= 0 and idx < _items.size():
		return _items[idx].id
	return -1


func select(idx: int) -> void:
	if idx >= 0 and idx < _items.size():
		selected = idx
	elif idx < 0:
		selected = -1
	if is_node_ready():
		_update_selected_text()


# -- Minimum size ------------------------------------------------------------


func _get_minimum_size() -> Vector2:
	if _vbox:
		return _vbox.get_combined_minimum_size()
	return Vector2.ZERO


# -- Popup control -----------------------------------------------------------


func _toggle_popup():
	if _is_open:
		_close_popup()
	else:
		_open_popup()


func _open_popup():
	_is_open = true
	_sync_popup_items()

	# Cover the full viewport so clicks outside close the popup
	_popup_layer.position = Vector2.ZERO
	_popup_layer.size = get_viewport().get_visible_rect().size

	# Position the popup panel just below the button, with 4px gap
	var button_global_pos := _button_panel.get_global_position()
	var button_size := _button_panel.size
	_popup_panel.position = Vector2(
		button_global_pos.x,
		button_global_pos.y + button_size.y + 4
	)
	_popup_panel.size.x = button_size.x

	# Constrain scroll height to max_popup_height
	var items_height := _items_container.get_combined_minimum_size().y
	_scroll_container.custom_minimum_size.y = min(max_popup_height, items_height)

	_popup_layer.visible = true
	_arrow_icon.flip_v = true
	_button_panel.add_theme_stylebox_override("panel", _style_pressed)


func _close_popup():
	_is_open = false
	_popup_layer.visible = false
	_arrow_icon.flip_v = false
	_button_panel.add_theme_stylebox_override("panel", _style_normal)


func _sync_popup_items():
	for child in _items_container.get_children():
		_items_container.remove_child(child)
		child.queue_free()

	for i in _items.size():
		var item: DropdownItem = DROPDOWN_ITEM_SCENE.instantiate()
		item.setup(i, _items[i].text, i == selected)
		item.pressed.connect(_on_item_pressed.bind(i))
		_items_container.add_child(item)


# -- Property updates --------------------------------------------------------


func _update_title():
	if _title_label:
		_title_label.text = title
		_title_label.visible = not title.is_empty()
		update_minimum_size()


func _update_description():
	if _description_label:
		_description_label.text = description
		_description_label.visible = not description.is_empty()
		update_minimum_size()


func _update_selected_text():
	if _selected_label:
		if selected >= 0 and selected < _items.size():
			_selected_label.text = _items[selected].text
			_selected_label.label_settings = load("res://assets/themes/selected_dropdown_settings.tres")
		else:
			_selected_label.text = "Select an option"
			_selected_label.label_settings = load("res://assets/themes/unselected_dropdown_settings.tres")

# -- Callbacks ---------------------------------------------------------------


func _on_button_gui_input(event: InputEvent):
	if event is InputEventMouseButton and event.pressed and event.button_index == MOUSE_BUTTON_LEFT:
		_toggle_popup()
		get_viewport().set_input_as_handled()


func _on_button_mouse_entered():
	if not _is_open:
		_button_panel.add_theme_stylebox_override("panel", _style_hover)


func _on_button_mouse_exited():
	if not _is_open:
		_button_panel.add_theme_stylebox_override("panel", _style_normal)


func _on_popup_layer_gui_input(event: InputEvent):
	if event is InputEventMouseButton and event.pressed:
		_close_popup()
		get_viewport().set_input_as_handled()


func _on_item_pressed(index: int):
	select(index)
	item_selected.emit(index)
	_close_popup()
