class_name SceneNametag
extends Control

## The pill a scene paints above an avatar's native nametag (#2811, `AvatarNametag`
## component). Drawn by NameplateLayer, which owns its position: the plate never
## positions or animates itself.

const FONT: FontFile = preload("res://assets/themes/fonts/inter/Inter-Bold.ttf")
# Scaled with the nameplate (NameplateLayer.SCALE), so these are nickname_ui units:
# the plate reads as ~70% of the name's font, as in the design-system tag.
const FONT_SIZE := 50
const PADDING := Vector2(18, 8)
const BORDER_WIDTH := 4
# Long scene labels truncate instead of stretching the pill off the avatar.
const MAX_WIDTH := 900.0

var label_text := ""

var _panel: Panel
var _label: Label
var _style: StyleBoxFlat


func _init() -> void:
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	_style = StyleBoxFlat.new()
	_panel = Panel.new()
	_panel.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_panel.add_theme_stylebox_override(&"panel", _style)
	_panel.set_anchors_preset(Control.PRESET_FULL_RECT)
	add_child(_panel)

	_label = Label.new()
	_label.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_label.auto_translate_mode = Node.AUTO_TRANSLATE_MODE_DISABLED
	_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	_label.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
	_label.text_overrun_behavior = TextServer.OVERRUN_TRIM_ELLIPSIS
	_label.clip_text = true
	_label.add_theme_font_override(&"font", FONT)
	_label.add_theme_font_size_override(&"font_size", FONT_SIZE)
	_label.set_anchors_preset(Control.PRESET_FULL_RECT)
	add_child(_label)


## Scene-authored values, already resolved against the native nametag colors by the
## Rust side. An empty label is a valid plate: it draws color only.
func configure(
	text: String, text_color: Color, background_color: Color, border_color: Color
) -> void:
	label_text = text
	_label.text = text
	_label.add_theme_color_override(&"font_color", text_color)
	_style.bg_color = background_color
	_style.border_color = border_color
	_style.set_border_width_all(BORDER_WIDTH)
	_sync_size()


## A label-less plate is a disc: keep it square and clamp the text width so the
## corners stay circular (the radius is half the height).
func _sync_size() -> void:
	var height := FONT.get_height(FONT_SIZE) + PADDING.y * 2.0
	var width := height
	if not label_text.is_empty():
		width = clampf(_text_width() + PADDING.x * 2.0, height, MAX_WIDTH)
	custom_minimum_size = Vector2(width, height)
	_style.set_corner_radius_all(int(height * 0.5))
	size = custom_minimum_size


## Measured per character: `get_string_size` collapses a spaces-only label to zero,
## and spaces are exactly how a scene widens a text-less plate (protocol#474).
func _text_width() -> float:
	var total := 0.0
	for index in label_text.length():
		total += FONT.get_char_size(label_text.unicode_at(index), FONT_SIZE).x
	return total
