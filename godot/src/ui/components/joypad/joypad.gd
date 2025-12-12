extends Control

var combo_opened: bool = false

@onready var animation_player: AnimationPlayer = %AnimationPlayer
@onready var button_combo: Button = %Button_Combo


func _on_button_combo_toggled(toggled_on: bool) -> void:
	combo_opened = toggled_on
	if toggled_on:
		animation_player.play("open_combo")
	else:
		animation_player.play_backwards("open_combo")


func _on_gui_input(event: InputEvent) -> void:
	if event is InputEventScreenTouch and combo_opened:
		UiSounds.play_sound("widget_emotes_close")
		animation_player.play_backwards("open_combo")
		button_combo.set_pressed_no_signal(false)
