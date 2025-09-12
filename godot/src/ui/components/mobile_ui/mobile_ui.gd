extends Control

@onready var button_mic: Control = %Button_Mic
@onready var virtual_joystick: VirtualJoystick = %VirtualJoystick_Left
@onready var button_primary_action: Control = %Button_PrimaryAction
@onready var button_secondary_action: Control = %Button_SecondaryAction
@onready var button_emote_action: Control = %Button_EmoteAction
@onready var button_jump: TextureButton = %Button_Jump


func _on_texture_button_toggled(toggled_on: bool) -> void:
	if toggled_on:
		button_mic.show()
		virtual_joystick.show()
		button_primary_action.show()
		button_secondary_action.show()
		button_emote_action.show()
		button_jump.show()
	else:
		button_mic.hide()
		virtual_joystick.hide()
		button_primary_action.hide()
		button_secondary_action.hide()
		button_emote_action.hide()
		button_jump.hide()
