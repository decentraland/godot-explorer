@tool
extends Control
class_name SearchBar

signal text_changed(new_text)
signal text_submitted(text)

@onready var line_edit: LineEdit = %LineEdit
@onready var button_erase_text: Button = %Button_EraseText
@onready var search_bar_panel_container: PanelContainer = %SearchBar_PanelContainer
@onready var button_search: Button = %Button_Search

var _text: String = ""
var _placeholder_text: String = ""
var _editable: bool = true

@export var text: String:
	get:
		return line_edit.text if line_edit else _text
	set(value):
		_text = value
		if line_edit:
			line_edit.text = value

@export var placeholder_text: String:
	get:
		return line_edit.placeholder_text if line_edit else _placeholder_text
	set(value):
		_placeholder_text = value
		if line_edit:
			line_edit.placeholder_text = value

@export var editable: bool = true:
	get:
		return line_edit.editable if line_edit else _editable
	set(value):
		_editable = value
		if line_edit:
			line_edit.editable = value
			
@export var closed: bool = true:
	get:
		return closed
	set(value):
		closed = value



func _ready() -> void:
	button_erase_text.hide()
	line_edit.text = _text
	line_edit.placeholder_text = _placeholder_text
	line_edit.editable = _editable
	line_edit.text_changed.connect(_on_text_changed)
	line_edit.text_submitted.connect(_on_text_submitted)
	line_edit.focus_entered.connect(_on_focus_entered)
	line_edit.focus_exited.connect(_on_focus_exited)
	close_searchbar()
	

func _on_text_changed(new_text: String) -> void:
	text_changed.emit(new_text)
	if new_text.length() <= 0:
		button_erase_text.hide()
		return
	button_erase_text.show()

func _on_text_submitted(submitted_text: String) -> void:
	text_submitted.emit(submitted_text)

func _on_focus_entered() -> void:
	focus_entered.emit()

func _on_focus_exited() -> void:
	focus_exited.emit()


func _on_button_erase_text_pressed() -> void:
	line_edit.clear()


func _on_button_search_pressed() -> void:
	open_searchbar()


func close_searchbar() -> void:
	closed = true
	line_edit.hide()
	line_edit.clear()
	button_erase_text.hide()
	button_search.show()
	search_bar_panel_container.self_modulate = Color.TRANSPARENT
	search_bar_panel_container.set_anchors_preset(Control.PRESET_CENTER_RIGHT)
	
func open_searchbar() -> void:
	closed = false
	button_search.hide()
	search_bar_panel_container.self_modulate = Color.WHITE
	search_bar_panel_container.set_anchors_preset(Control.PRESET_HCENTER_WIDE)
