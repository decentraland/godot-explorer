extends Control

var combo_opened: bool = false

@onready var animation_player: AnimationPlayer = %AnimationPlayer
@onready var button_combo: Button = %Button_Combo


func _ready() -> void:
	Global.close_combo.connect(_close_because_action)


func _on_button_combo_toggled(toggled_on: bool) -> void:
	combo_opened = toggled_on
	if toggled_on:
		animation_player.play("open_combo")
		UiSounds.play_sound("widget_emotes_open")
	else:
		animation_player.play_backwards("open_combo")
		UiSounds.play_sound("widget_emotes_close")


func _close_because_action() -> void:
	if combo_opened == true:
		button_combo.toggled.emit(false)
		button_combo.set_pressed_no_signal(false)
