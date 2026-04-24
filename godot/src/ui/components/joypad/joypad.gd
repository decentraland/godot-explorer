extends Control

const GLIDER_ICON_MAX_WIDTH = 85
const GLIDER_ICON = preload("uid://dnosnq2stqu11")  # "res://assets/themes/dark_dcl_theme/icons/Glider.svg"

const DOUBLE_JUMP_ICON_MAX_WIDTH = 52
const DOUBLE_JUMP_ICON = preload("uid://euvimxirt85b")  # "res://assets/themes/dark_dcl_theme/icons/DoubleJump.svg"

const TOUCHABLE_NORMAL_STYLEBOX = preload("uid://b66geet5bo5yf")  # touchable_normal.tres (black bg)
const TOUCHABLE_PRESSED_STYLEBOX = preload("uid://cvducxvis7n6e")  # touchable_pressed.tres (white bg)

const TOUCHABLE_ICON_LIGHT := Color(0.9882353, 0.9882353, 0.9882353, 1)
const TOUCHABLE_ICON_DARK := Color(0, 0, 0, 0.7019608)

const INVERTED_NORMAL_STYLES: Array[StringName] = [
	&"normal", &"normal_mirrored", &"hover", &"hover_mirrored"
]
const INVERTED_PRESSED_STYLES: Array[StringName] = [
	&"pressed", &"pressed_mirrored", &"hover_pressed", &"hover_pressed_mirrored"
]
const INVERTED_NORMAL_ICON_COLORS: Array[StringName] = [&"icon_normal_color", &"icon_hover_color"]
const INVERTED_PRESSED_ICON_COLORS: Array[StringName] = [
	&"icon_pressed_color", &"icon_hover_pressed_color"
]

var combo_opened: bool = false

var _showing_glider_icon: bool = false
var _showing_inverted_colors: bool = false

@onready var animation_player: AnimationPlayer = %AnimationPlayer
@onready var button_combo: Button = %Button_Combo
@onready var button_press: Button = $Button_Press

@onready var _combo_action_buttons: Array[Button] = [
	%Button_Combo1,
	%Button_Combo2,
	%Button_Combo3,
	%Button_Combo4,
]


func _ready() -> void:
	for btn in _combo_action_buttons:
		btn.touch_action_changed.connect(_on_combo_action_changed)
	_set_attenuated_sound_for_buttons(self)
	_apply_jump_icon(false)


func _process(_dt: float) -> void:
	_update_jump_icon()


func _update_jump_icon() -> void:
	var player := Global.scene_runner.player_body_node as Player
	if player == null:
		return

	# Pick the icon from the player's own decision tree so it matches the
	# action taken. NONE keeps the current icon to avoid flicker during
	# transient states like GLIDE_CLOSING or the post-close cooldown.
	var want_glider := _showing_glider_icon
	match player.get_jump_action():
		Player.JUMP_ACTION_JUMP:
			want_glider = false
		Player.JUMP_ACTION_GLIDE_TOGGLE:
			want_glider = true
	if want_glider != _showing_glider_icon:
		_apply_jump_icon(want_glider)

	var glide_active := (
		player.glide_state != Player.GLIDE_CLOSED
		and not (player.is_on_floor() or player.position.y <= 0.0)
	)
	if glide_active != _showing_inverted_colors:
		_showing_inverted_colors = glide_active
		_apply_inverted_colors(glide_active)


func _apply_jump_icon(show_glider: bool) -> void:
	_showing_glider_icon = show_glider
	if show_glider:
		button_press.icon = GLIDER_ICON
		button_press.add_theme_constant_override("icon_max_width", GLIDER_ICON_MAX_WIDTH)
	else:
		button_press.icon = DOUBLE_JUMP_ICON
		button_press.add_theme_constant_override("icon_max_width", DOUBLE_JUMP_ICON_MAX_WIDTH)


func _apply_inverted_colors(inverted: bool) -> void:
	if inverted:
		for style in INVERTED_NORMAL_STYLES:
			button_press.add_theme_stylebox_override(style, TOUCHABLE_PRESSED_STYLEBOX)
		for style in INVERTED_PRESSED_STYLES:
			button_press.add_theme_stylebox_override(style, TOUCHABLE_NORMAL_STYLEBOX)
		for color_name in INVERTED_NORMAL_ICON_COLORS:
			button_press.add_theme_color_override(color_name, TOUCHABLE_ICON_DARK)
		for color_name in INVERTED_PRESSED_ICON_COLORS:
			button_press.add_theme_color_override(color_name, TOUCHABLE_ICON_LIGHT)
	else:
		for style in INVERTED_NORMAL_STYLES:
			button_press.remove_theme_stylebox_override(style)
		for style in INVERTED_PRESSED_STYLES:
			button_press.remove_theme_stylebox_override(style)
		for color_name in INVERTED_NORMAL_ICON_COLORS:
			button_press.remove_theme_color_override(color_name)
		for color_name in INVERTED_PRESSED_ICON_COLORS:
			button_press.remove_theme_color_override(color_name)


func _set_attenuated_sound_for_buttons(node: Node) -> void:
	if node is Button:
		node.set_meta("attenuated_sound", true)

	for child in node.get_children():
		_set_attenuated_sound_for_buttons(child)


func _on_button_combo_toggled(toggled_on: bool) -> void:
	combo_opened = toggled_on
	if toggled_on:
		animation_player.play("open_combo")
	else:
		animation_player.play_backwards("open_combo")


func _on_combo_action_changed(pressed: bool) -> void:
	if not pressed and combo_opened:
		button_combo.toggled.emit(false)
		button_combo.set_pressed_no_signal(false)
