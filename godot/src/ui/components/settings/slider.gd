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
		if is_node_ready() and _slider:
			_slider.value = val

## Minimum value of the slider.
@export var min_value: float = 0.0:
	set(val):
		min_value = val
		if is_node_ready() and _slider:
			_slider.min_value = val

## Maximum value of the slider.
@export var max_value: float = 100.0:
	set(val):
		max_value = val
		if is_node_ready() and _slider:
			_slider.max_value = val

## Step increment of the slider.
@export var step: float = 1.0:
	set(val):
		step = val
		if is_node_ready() and _slider:
			_slider.step = val

## Whether the slider is editable.
@export var editable: bool = true:
	set(val):
		editable = val
		if is_node_ready() and _slider:
			_slider.editable = val

@onready var _title_label: Label = %Label
@onready var _slider: HSlider = %HSlider_GeneralVolume


func _ready():
	_update_title()

	if Engine.is_editor_hint():
		return

	_slider.min_value = min_value
	_slider.max_value = max_value
	_slider.step = step
	_slider.value = value
	_slider.editable = editable

	_slider.value_changed.connect(_on_slider_value_changed)
	_slider.drag_ended.connect(_on_slider_drag_ended)


func _update_title():
	if _title_label:
		_title_label.text = title


func _on_slider_value_changed(new_value: float):
	value = new_value
	value_changed.emit(new_value)


func _on_slider_drag_ended(changed: bool):
	drag_ended.emit(changed)
