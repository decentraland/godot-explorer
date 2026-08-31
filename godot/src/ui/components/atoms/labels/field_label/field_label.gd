@tool
class_name FieldLabel
extends RichTextLabel

## Form field caption, optionally with a required-marker asterisk.
##
## RichTextLabel rather than Label because the asterisk is a different colour and
## weight from the caption, which `LabelSettings` cannot express in one node.
##
## The colours are the design system's: captions are Snow (#FCFCFC) and the
## required marker is the same red as DclTextEdit's error label (#FF2D55), so a
## required field and its validation message agree visually.

const COLOR_TEXT := "#fcfcfc"
const COLOR_REQUIRED := "#ff2d55"

@export var text_value: String = "Label":
	set(value):
		text_value = value
		_refresh()

@export var required: bool = false:
	set(value):
		required = value
		_refresh()

@export var font_size: int = 30:
	set(value):
		font_size = value
		_refresh()


func _ready() -> void:
	_refresh()


func _refresh() -> void:
	if not is_node_ready():
		return
	var marker := "[color=%s][b]*[/b][/color]" % COLOR_REQUIRED if required else ""
	text = (
		"[font_size=%d][color=%s]%s[/color]%s[/font_size]"
		% [font_size, COLOR_TEXT, text_value, marker]
	)
