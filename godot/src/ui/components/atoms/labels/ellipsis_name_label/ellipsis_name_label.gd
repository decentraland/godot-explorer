@tool
class_name EllipsisNameLabel
extends HBoxContainer

## Attach to an HBoxContainer that holds a single-line [Label] (with clip_text on) optionally
## followed by a trailing icon. It caps the label to the width this container gets from its parent,
## so a short name hugs its text — keeping the trailing icon glued to it — while a long name clips
## with an ellipsis. clip_text zeroes the label's own minimum width, so the fit can't be driven by
## container size flags and is measured here instead.
##
## The label must NOT expand (size_flags_horizontal without SIZE_EXPAND) or it would stretch past
## its text and unglue the icon. Call [method refresh] after changing the label text or toggling
## the trailing icon, since neither emits a resize.

@export var label: Label:
	set(value):
		label = value
		if is_node_ready():
			_update_width()

## Optional node kept to the right of the label (e.g. a claimed-name check). Its width — while
## visible — is reserved so the ellipsis never runs underneath it.
@export var trailing: Control


func _ready() -> void:
	resized.connect(_update_width)
	_update_width()


## Re-measures and re-applies the width cap. Call after the label text or the trailing icon's
## visibility changes.
func refresh() -> void:
	_update_width()


func _update_width() -> void:
	if label == null:
		return

	var font: Font
	var font_size: int
	if label.label_settings != null:
		font = label.label_settings.font
		font_size = label.label_settings.font_size
	else:
		font = label.get_theme_font(&"font")
		font_size = label.get_theme_font_size(&"font_size")
	if font == null:
		return

	var content_width: float = ceilf(
		font.get_string_size(label.text, HORIZONTAL_ALIGNMENT_LEFT, -1, font_size).x
	)
	# Leave room for the text outline so the last glyph never clips its stroke.
	content_width += label.get_theme_constant(&"outline_size") * 2.0

	var reserved: float = 0.0
	if trailing != null and trailing.visible:
		reserved = trailing.get_combined_minimum_size().x + get_theme_constant(&"separation")

	var available: float = size.x - reserved
	label.custom_minimum_size.x = clampf(content_width, 0.0, maxf(0.0, available))
