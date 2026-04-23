extends Control

const GLIDER_ICON_MAX_WIDTH = 85
const GLIDER_ICON = preload("uid://dnosnq2stqu11")  # "res://assets/themes/dark_dcl_theme/icons/Glider.svg"

const DOUBLE_JUMP_ICON_MAX_WIDTH = 52
const DOUBLE_JUMP_ICON = preload("uid://euvimxirt85b")  # "res://assets/themes/dark_dcl_theme/icons/DoubleJump.svg"

var combo_opened: bool = false

var _showing_glider_icon: bool = false

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
	var show_glider := player != null and player.can_toggle_glide()
	if show_glider == _showing_glider_icon:
		return
	_apply_jump_icon(show_glider)


func _apply_jump_icon(show_glider: bool) -> void:
	_showing_glider_icon = show_glider
	if show_glider:
		button_press.icon = GLIDER_ICON
		button_press.add_theme_constant_override("icon_max_width", GLIDER_ICON_MAX_WIDTH)
	else:
		button_press.icon = DOUBLE_JUMP_ICON
		button_press.add_theme_constant_override("icon_max_width", DOUBLE_JUMP_ICON_MAX_WIDTH)


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
