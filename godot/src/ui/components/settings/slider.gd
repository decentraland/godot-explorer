@tool
class_name SettingsSlider
extends VBoxContainer

signal value_changed(value: float)
signal drag_ended(value_changed: bool)

## Title displayed above the slider.
@export var title: String = "":
	set(val):
		title = val
		if is_node_ready():
			_update_title()

## Current value of the slider.
@export var value: float = 50.0:
	set(val):
		value = val
		if is_node_ready() and _h_slider:
			_h_slider.value = val
		if is_node_ready():
			_update_value_label()

## Minimum value of the slider.
@export var min_value: float = 0.0:
	set(val):
		min_value = val
		if is_node_ready() and _h_slider:
			_h_slider.min_value = val

## Maximum value of the slider.
@export var max_value: float = 100.0:
	set(val):
		max_value = val
		if is_node_ready() and _h_slider:
			_h_slider.max_value = val

## Step increment of the slider.
@export var step: float = 1.0:
	set(val):
		step = val
		if is_node_ready() and _h_slider:
			_h_slider.step = val

## Whether the slider is editable.
@export var editable: bool = true:
	set(val):
		editable = val
		if is_node_ready() and _h_slider:
			_h_slider.editable = val

## If true, the value label displays a percentage (e.g. "7%5").
## If false, it displays the absolute value (e.g. "75").
@export var is_percentage: bool = false:
	set(val):
		is_percentage = val
		if is_node_ready():
			_update_value_label()

@onready var _title_label: Label = %Label
@onready var _value_label: Label = %ValueLabel
@onready var _h_slider: HSlider = %HSlider


func _ready():
	_update_title()
	_update_value_label()

	if Engine.is_editor_hint():
		return

	_h_slider.min_value = min_value
	_h_slider.max_value = max_value
	_h_slider.step = step
	_h_slider.value = value
	_h_slider.editable = editable

	_h_slider.value_changed.connect(_on_h_slider_value_changed)
	_h_slider.drag_ended.connect(_on_h_slider_drag_ended)


func _update_title():
	if _title_label:
		_title_label.text = title


func _update_value_label():
	if _value_label:
		var display_value := int(value)
		if is_percentage:
			display_value = int(value / max_value * 100)
			_value_label.text = str(display_value) + "%"
		else:
			_value_label.text = str(display_value)


func _on_h_slider_value_changed(new_value: float):
	value = new_value
	value_changed.emit(new_value)


func _on_h_slider_drag_ended(changed: bool):
	drag_ended.emit(changed)
