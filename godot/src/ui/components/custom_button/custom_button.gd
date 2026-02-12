@tool
class_name CustomButton
extends Button

enum IconAlign { LEFT, RIGHT }

@export_group("Icon")

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

@export var icon_normal_color: Color = Color.WHITE:
	set(value):
		icon_normal_color = value
		if is_node_ready():
			_update_visual_state()

@export var icon_pressed_color: Color = Color.WHITE:
	set(value):
		icon_pressed_color = value
		if is_node_ready():
			_update_visual_state()

@export var icon_disabled_color: Color = Color.WHITE:
	set(value):
		icon_disabled_color = value
		if is_node_ready():
			_update_visual_state()

@export var icon_align: IconAlign = IconAlign.LEFT:
	set(value):
		icon_align = value
		if is_node_ready():
			_update_icon_align()

@export_group("Text")

@export var custom_text: String:
	set(value):
		custom_text = value
		if is_node_ready():
			_update_text()

@export var text_normal_settings: LabelSettings:
	set(value):
		text_normal_settings = value
		if is_node_ready():
			_update_visual_state()

@export var text_pressed_settings: LabelSettings:
	set(value):
		text_pressed_settings = value
		if is_node_ready():
			_update_visual_state()

@export var text_disabled_settings: LabelSettings:
	set(value):
		text_disabled_settings = value
		if is_node_ready():
			_update_visual_state()

@onready var _hbox: HBoxContainer = %HBoxContainer_Content
@onready var _icon: TextureRect = %TextureRect_Icon
@onready var _label: Label = %Label_Text


func _ready():
	_update_text()
	_update_icon_align()
	_update_visual_state()


func _notification(what: int) -> void:
	if what == NOTIFICATION_DRAW:
		_update_visual_state()


func _update_text():
	if _label:
		_label.text = custom_text


func _update_visual_state():
	if not _label or not _icon:
		return

	var mode := get_draw_mode()
	match mode:
		DRAW_DISABLED:
			if text_disabled_settings:
				_label.label_settings = text_disabled_settings
			_icon.texture = custom_icon
			_icon.modulate = icon_disabled_color
		DRAW_PRESSED, DRAW_HOVER_PRESSED:
			if text_pressed_settings:
				_label.label_settings = text_pressed_settings
			_icon.texture = custom_icon_pressed if custom_icon_pressed else custom_icon
			_icon.modulate = icon_pressed_color
		_:
			if text_normal_settings:
				_label.label_settings = text_normal_settings
			_icon.texture = custom_icon
			_icon.modulate = icon_normal_color

	_icon.visible = _icon.texture != null


func _update_icon_align():
	if _icon and _hbox:
		match icon_align:
			IconAlign.LEFT:
				_hbox.move_child(_icon, 0)
			IconAlign.RIGHT:
				_hbox.move_child(_icon, _hbox.get_child_count() - 1)
