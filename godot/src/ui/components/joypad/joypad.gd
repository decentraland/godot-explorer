extends Control

var combo_opened: bool = false

@onready var animation_player: AnimationPlayer = %AnimationPlayer
@onready var button_combo: Button = %Button_Combo
@onready var button_press: Button = %Button_Press
@onready var button_jump: Button = %Button_Jump
@onready var button_primary: Button = %Button_Primary
@onready var button_secondary: Button = %Button_Secondary
@onready var button_combo_1: Button = %Button_Combo1
@onready var button_combo_2: Button = %Button_Combo2
@onready var button_combo_3: Button = %Button_Combo3
@onready var button_combo_4: Button = %Button_Combo4


func _ready() -> void:
	Global.close_combo.connect(_close_because_action)
	button_combo.set_meta("attenuated_sound", true)
	button_press.set_meta("attenuated_sound", true)
	button_jump.set_meta("attenuated_sound", true)
	button_primary.set_meta("attenuated_sound", true)
	button_secondary.set_meta("attenuated_sound", true)
	button_combo_1.set_meta("attenuated_sound", true)
	button_combo_2.set_meta("attenuated_sound", true)
	button_combo_3.set_meta("attenuated_sound", true)
	button_combo_4.set_meta("attenuated_sound", true)


func _on_button_combo_toggled(toggled_on: bool) -> void:
	combo_opened = toggled_on
	if toggled_on:
		animation_player.play("open_combo")
	else:
		animation_player.play_backwards("open_combo")


func _close_because_action() -> void:
	if combo_opened == true:
		button_combo.toggled.emit(false)
		button_combo.set_pressed_no_signal(false)
