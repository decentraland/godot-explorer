extends Control

var combo_opened: bool = false


@onready var animation_player: AnimationPlayer = %AnimationPlayer
@onready var button_combo: Button = %Button_Combo


func _on_button_combo_toggled(toggled_on: bool) -> void:
	if toggled_on:
		animation_player.play("open_combo")
		UiSounds.play_sound("widget_emotes_open")
	else:
		animation_player.play_backwards("open_combo")
		UiSounds.play_sound("widget_emotes_close")
		
