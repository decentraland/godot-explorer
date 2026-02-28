@tool
class_name CustomButton
extends Button

enum IconAlign { LEFT, RIGHT, CENTER_LEFT, CENTER_RIGHT }

@export var custom_icon: Texture2D:
	set(value):
		custom_icon = value
		if is_node_ready():
			_update_visual_state()

@export var custom_icon_pressed: Texture2D:
	set(value):
		custom_icon_pressed = value
		if is_node_ready():
			_update_visual_state()

@export var custom_text: String:
	set(value):
		custom_text = value
		if is_node_ready():
			_update_text()

@export var icon_align: IconAlign = IconAlign.CENTER_LEFT:
	set(value):
		icon_align = value
		if is_node_ready():
			_update_icon_align()

@onready var _margin: MarginContainer = %MarginContainer
@onready var _hbox: HBoxContainer = %HBoxContainer_Content
@onready var _icon: TextureRect = %TextureRect_Icon
@onready var _spacer: VSeparator = %VSeparator_Spacer
@onready var _label: Label = %Label_Text


func _ready():
	_update_text()
	_update_icon_align()
	_update_visual_state()


func _notification(what: int) -> void:
	if what == NOTIFICATION_DRAW or what == NOTIFICATION_THEME_CHANGED:
		_update_visual_state()


func _update_text():
	if _label:
		_label.text = custom_text


func _update_visual_state():
	if not _label or not _icon or not _spacer or not _margin:
		return

	# Font from theme (shared across all states)
	_label.add_theme_font_override("font", get_theme_font("font"))
	_label.add_theme_font_size_override("font_size", get_theme_font_size("font_size"))

	# h_separation from theme
	_hbox.add_theme_constant_override("separation", get_theme_constant("h_separation"))

	# State-dependent stylebox, colors, and icon texture
	var font_color: Color
	var icon_color: Color
	var stylebox_name: String
	var mode := get_draw_mode()

	match mode:
		DRAW_DISABLED:
			font_color = get_theme_color("font_disabled_color")
			icon_color = get_theme_color("icon_disabled_color")
			_icon.texture = custom_icon
			stylebox_name = "disabled"
		DRAW_HOVER_PRESSED:
			font_color = get_theme_color("font_hover_pressed_color")
			icon_color = get_theme_color("icon_hover_pressed_color")
			_icon.texture = custom_icon_pressed if custom_icon_pressed else custom_icon
			stylebox_name = "pressed"
		DRAW_PRESSED:
			font_color = get_theme_color("font_pressed_color")
			icon_color = get_theme_color("icon_pressed_color")
			_icon.texture = custom_icon_pressed if custom_icon_pressed else custom_icon
			stylebox_name = "pressed"
		DRAW_HOVER:
			font_color = get_theme_color("font_hover_color")
			icon_color = get_theme_color("icon_hover_color")
			_icon.texture = custom_icon
			stylebox_name = "hover"
		_:
			font_color = get_theme_color("font_color")
			icon_color = get_theme_color("icon_normal_color")
			_icon.texture = custom_icon
			stylebox_name = "normal"

	_label.add_theme_color_override("font_color", font_color)
	_icon.modulate = icon_color
	_icon.visible = _icon.texture != null

	# Icon min size based on font size
	var fs := get_theme_font_size("font_size")
	_icon.custom_minimum_size.x = int(fs * 1.1)

	# Margins from the active stylebox
	var style := get_theme_stylebox(stylebox_name)
	if style:
		_margin.add_theme_constant_override("margin_left", int(style.content_margin_left))
		_margin.add_theme_constant_override("margin_top", int(style.content_margin_top))
		_margin.add_theme_constant_override("margin_right", int(style.content_margin_right))
		_margin.add_theme_constant_override("margin_bottom", int(style.content_margin_bottom))

	_update_min_size.call_deferred()


func _update_min_size():
	if not _margin:
		return
	custom_minimum_size.x = _margin.get_combined_minimum_size().x


func _update_icon_align():
	if not _icon or not _hbox or not _spacer or not _label:
		return

	match icon_align:
		IconAlign.LEFT:
			_hbox.move_child(_icon, 0)
			_hbox.move_child(_spacer, 1)
			_hbox.move_child(_label, 2)
			_spacer.visible = true
			_spacer.size_flags_horizontal = Control.SIZE_EXPAND_FILL
			_hbox.alignment = BoxContainer.ALIGNMENT_BEGIN
		IconAlign.RIGHT:
			_hbox.move_child(_label, 0)
			_hbox.move_child(_spacer, 1)
			_hbox.move_child(_icon, 2)
			_spacer.visible = true
			_spacer.size_flags_horizontal = Control.SIZE_EXPAND_FILL
			_hbox.alignment = BoxContainer.ALIGNMENT_BEGIN
		IconAlign.CENTER_LEFT:
			_hbox.move_child(_icon, 0)
			_hbox.move_child(_spacer, 1)
			_hbox.move_child(_label, 2)
			_spacer.visible = false
			_hbox.alignment = BoxContainer.ALIGNMENT_CENTER
		IconAlign.CENTER_RIGHT:
			_hbox.move_child(_label, 0)
			_hbox.move_child(_spacer, 1)
			_hbox.move_child(_icon, 2)
			_spacer.visible = false
			_hbox.alignment = BoxContainer.ALIGNMENT_CENTER
